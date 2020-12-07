use clap::Clap;
use bitcoin_cash::*;
use anyhow::{Context, Result};

use crate::contract::*;
use crate::ecs_client::*;

#[derive(Clap)]
pub struct SendHtlc {
    #[clap(long)]
    token_id: String,
    #[clap(long)]
    amount: String,
    #[clap(long)]
    seller_address: String,
    #[clap(long)]
    secret_hash: String,
    #[clap(long)]
    timeout: u32,
    #[clap(long)]
    uri: String,
}

impl SendHtlc {
    pub fn run(&self, prefix: &str) -> Result<()> {
        let client = ECSClient::new(self.uri.clone(), prefix);
        let buyer_address = client.createaddress().with_context(|| "Couldnt create buyer address")?;
        let seller_address = Address::from_cash_addr(&self.seller_address).with_context(
            || "Invalid seller address: {}"
        )?;
        if seller_address.prefix_str() != prefix {
            anyhow::bail!("Seller address must start with {}.", prefix)
        }
        if seller_address.addr_type() != AddressType::P2PKH {
            anyhow::bail!("Seller address must be P2PKH")
        }
        let secret_hash = Hash160::from_hex_be(&self.secret_hash).with_context(
            || format!("Invalid secret hash")
        )?;
        let params = SlpHtlcParams {
            seller_pkh: seller_address.hash().clone(),
            buyer_pkh: buyer_address.hash().clone(),
            secret_hash,
            timeout: Integer::new(self.timeout)
                .with_context(|| format!("Invalid timeout: {}", self.timeout))?,
        };
        let script = params.script();
        let p2sh_address = Address::from_redeem_script(prefix, script.into()).expect("infallible");
        let p2sh: Script = p2sh_address.clone().into();
        let tx_hex = client.payto_slp(&self.token_id, &self.amount, p2sh_address.cash_addr())?;
        let tx_hex = client.signtransaction(&tx_hex)?;
        let tx_hash = client.broadcast(&tx_hex)?;
        let raw_tx = hex::decode(&tx_hex)?;
        let (tx, _): (UnhashedTx, _) = UnhashedTx::deser(raw_tx.clone().into())?;
        for (idx, output) in tx.outputs.iter().enumerate() {
            if output.script.ser_ops() == p2sh.ser_ops() {
                println!("buyer address: {}", buyer_address.cash_addr());
                println!("timeout: {}", self.timeout);
                println!("contract UTXO: {}:{}", tx_hash, idx);
                return Ok(());
            }
        }
        anyhow::bail!("Invalid tx {}, could not find {}.", tx_hex, p2sh.ser_ops().hex());
    }
}