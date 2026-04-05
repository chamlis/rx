use std::process::Stdio;

use anyhow::{Context, Result};
use futures_concurrency::future::Race;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc::{self, Receiver, Sender};

use clap::Parser;

#[derive(Parser, Debug)]
#[command(about = "Experimental reactivity in the shell")]
struct Args {
    #[arg(
        help = "The output command, run with the latest line from each input command as its arguments."
    )]
    output_command: String,

    #[arg(
        required = true,
        allow_hyphen_values = true,
        help = "The input commands and their arguments, separated by ';', whose output lines are used to trigger and control the input command."
    )]
    input_commands: Vec<String>,
}

async fn input_task(mut cmd: Command, tx: Sender<String>) -> Result<()> {
    let mut child = cmd.spawn().context("error running input command")?;
    let stdout = child.stdout.take().unwrap();
    let mut reader = BufReader::new(stdout).lines();

    tokio::spawn(async move {
        child.wait().await.unwrap();
    });

    while let Some(line) = reader.next_line().await? {
        tx.send(line).await?;
    }

    Ok(())
}

async fn output_task(cmd: &str, mut channels: Vec<Receiver<String>>) -> Result<()> {
    let mut latest_args = vec![];
    for rx in &mut channels {
        latest_args.push(rx.recv().await.unwrap());
    }

    let mut channel_closed: Vec<bool> = channels.iter().map(|_| false).collect();

    loop {
        Command::new(cmd)
            .args(&latest_args)
            .status()
            .await
            .context("error running output command")?;

        let (changed_ix, new_arg) = loop {
            let fs = channels
                .iter_mut()
                .zip(channel_closed.iter())
                .enumerate()
                .filter(|(_, (_, closed))| !**closed)
                .map(|(ix, (rx, _))| async move { (ix, rx.recv().await) })
                .collect::<Vec<_>>();

            let (changed_ix, new_arg) = fs.race().await;

            match new_arg {
                Some(new_arg) if latest_args[changed_ix] != new_arg => break (changed_ix, new_arg),
                Some(_) => continue,
                None => {
                    channel_closed[changed_ix] = true;
                }
            }
        };

        latest_args[changed_ix] = new_arg;
    }
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let mut channels = vec![];

    for args in args.input_commands.split(|x| x == ";") {
        let mut c = Command::new(&args[0]);
        c.args(&args[1..]);
        c.stdout(Stdio::piped());

        // TODO: ideally we want a capacity of 0...
        let (tx, rx) = mpsc::channel(1);
        channels.push(rx);
        tokio::spawn(input_task(c, tx));
    }

    output_task(&args.output_command, channels).await.unwrap();
}
