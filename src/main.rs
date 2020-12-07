use bitcoin_cash::{Hash160, Hashed};
use clap::Clap;

mod contract;
mod ecs_client;
mod send_htlc;
mod redeem_htlc;
mod timeout_htlc;
mod util;

use send_htlc::*;
use redeem_htlc::*;
use timeout_htlc::*;

#[derive(Clap)]
#[clap(version = "0.1", author = "Tobias Ruck <contact@be.cash>")]
struct Opts {
    #[clap(subcommand)]
    cmd: HtlcCommand,
}

#[derive(Clap)]
enum HtlcCommand {
    SendHtlc(SendHtlc),
    RedeemHtlc(RedeemHtlc),
    TimeoutHtlc(TimeoutHtlc),
    GenSecret,
}

fn main() {
    let opts: Opts = Opts::parse();
    let prefix = "slptest";
    let result = match &opts.cmd {
        HtlcCommand::SendHtlc(make_htlc) => {
            make_htlc.run(prefix)
        }
        HtlcCommand::RedeemHtlc(redeem_htlc) => {
            redeem_htlc.run(prefix)
        }
        HtlcCommand::TimeoutHtlc(timeout_htlc) => {
            timeout_htlc.run(prefix)
        }
        HtlcCommand::GenSecret => {
            use rand::RngCore;
            let mut rng = rand::thread_rng();
            let mut secret = [0; 32];
            rng.fill_bytes(&mut secret);
            println!("secret: {}", hex::encode(&secret));
            println!("secret hash: {}", hex::encode(&Hash160::digest_slice(&secret)));
            Ok(())
        }
    };

    match result {
        Ok(()) => {}
        Err(err) => {
            eprintln!("Error creating HTLC:\n{:?}", err);
        }
    }
}
