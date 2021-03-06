use clap::Clap;
use bitcoin_cash::*;
use bitcoin_cash_ecc::init_ecc;
use bitcoin_cash_slp::{slp_send_output, SlpTokenType};
use anyhow::{Context, Result};

use crate::contract::*;
use crate::ecs_client::*;
use crate::util;

#[derive(Clap)]
pub struct RedeemHtlc {
    #[clap(long)]
    contract_utxo: String,
    #[clap(long)]
    buyer_address: String,
    #[clap(long)]
    secret: String,
    #[clap(long)]
    timeout: u32,
    #[clap(long)]
    seller_secret_key: Option<String>,
    #[clap(long)]
    seller_address: Option<String>,
    #[clap(long)]
    uri: String,
}

impl RedeemHtlc {
    pub fn run(&self, prefix: &str) -> Result<()> {
        let client = ECSClient::new(self.uri.clone(), prefix);
        let utxo_msg = "Invalid contract UTXO, must be of form <txid>:<vout>";
        let utxo_err = anyhow::anyhow!(utxo_msg);
        let mut contract_utxo_split = self.contract_utxo.splitn(2, ":");
        let contract_tx_hash_hex = contract_utxo_split.next().expect("infallible");
        let contract_vout = contract_utxo_split.next().ok_or_else(|| utxo_err)?;
        let contract_tx_hash = Sha256d::from_hex_le(contract_tx_hash_hex).with_context(|| utxo_msg)?;
        let contract_vout: u32 = contract_vout.parse().with_context(|| utxo_msg)?;
        let ecc = init_ecc();
        let (seller_address, seller_pk, seller_sk) = match (self.seller_secret_key.as_ref(), self.seller_address.as_ref()) {
            (Some(_), Some(_)) | (None, None) => {
                anyhow::bail!("Either seller secret key or seller address must be set, but not both.");
            }
            (Some(seller_secret_key), None) => {
                let seller_sk = hex::decode(seller_secret_key)
                    .with_context(|| "Invalid seller secret key")?;
                let seller_pk = ecc.derive_pubkey(&seller_sk)?;
                (Address::from_pk(prefix, &seller_pk), seller_pk, seller_sk)
            }
            (None, Some(seller_address)) => {
                let seller_address = Address::from_cash_addr(seller_address)
                    .with_context(|| "Invalid seller address")?;
                if seller_address.prefix_str() != prefix {
                    anyhow::bail!("Seller address must start with {}.", prefix)
                }
                if seller_address.addr_type() != AddressType::P2PKH {
                    anyhow::bail!("Seller address must be P2PKH")
                }
                let seller_sk = client.getprivatekeys(seller_address.cash_addr())
                    .with_context(|| format!("Address {} not part of wallet", seller_address.cash_addr()))?;
                let seller_pk = ecc.derive_pubkey(&seller_sk)?;
                (seller_address, seller_pk, seller_sk.to_vec())
            }
        };
        let buyer_address = Address::from_cash_addr(&self.buyer_address).with_context(
            || "Invalid buyer address: {}"
        )?;
        if buyer_address.prefix_str() != prefix {
            anyhow::bail!("Buyer address must start with {}.", prefix)
        }
        if buyer_address.addr_type() != AddressType::P2PKH {
            anyhow::bail!("Buyer address must be P2PKH")
        }
        let secret = hex::decode(&self.secret).with_context(|| "Invalid secret")?;
        let secret_hash = Hash160::digest(secret.clone());
        let timeout = Integer::new(self.timeout)
            .with_context(|| format!("Invalid timeout: {}", self.timeout))?;

        if !client.slpvalidate(contract_tx_hash_hex)? {
            anyhow::bail!("Contract tx is not a valid SLP transaction.");
        }

        let (token_id, contract_amount) = util::get_utxo_token_amount(&client, contract_tx_hash_hex, contract_vout)?;
        println!("contract_amount: {}", contract_amount);
        println!("token_id: {:?}", token_id);

        let params = SlpHtlcParams {
            buyer_pkh: buyer_address.hash().clone(),
            seller_pkh: seller_address.hash().clone(),
            secret_hash,
            timeout,
        };
        let recipient_address = client.createaddress()?;
        let recipient_script: Script = recipient_address.into();
        let mut tx_builder = TxBuilder::new_simple();
        let contract_ref = tx_builder.add_input(
            UnsignedTxInput {
                prev_out: TxOutpoint { tx_hash: contract_tx_hash, vout: contract_vout },
                sequence: 0xffff_ffff,
                value: DUST_AMOUNT,
            },
            params.script(),
            SlpHtlcSignatory::Redeem {
                secret: secret.into(),
                seller_pk
            },
        );

        tx_builder.add_output(
            slp_send_output(SlpTokenType::Fungible, &token_id, &[contract_amount])
        );

        tx_builder.add_output(TxOutput {
            script: recipient_script.clone(),
            value: DUST_AMOUNT,
        });

        tx_builder.add_leftover_output_bounded(0, u64::MAX, 0, recipient_script);

        let (mut unsigned_tx, gas_inputs) = util::add_gas_inputs(&client, &ecc, tx_builder)?;

        let contract_sig = ecc.sign(&seller_sk, Sha256d::digest(unsigned_tx.input_preimages(contract_ref).ser()))?;
        unsigned_tx.sign_input(contract_ref, contract_sig)?;
        for (gas_ref, utxo_sk) in gas_inputs {
            let gas_sig = ecc.sign(&utxo_sk, Sha256d::digest(unsigned_tx.input_preimages(gas_ref).ser()))?;
            unsigned_tx.sign_input(gas_ref, gas_sig)?;
        }

        let htlc_tx = unsigned_tx.complete_tx();
        let htlc_raw_tx = htlc_tx.ser();
        let htlc_tx_hex = hex::encode(&htlc_raw_tx);

        let tx_hash = client.broadcast(&htlc_tx_hex)
            .with_context(|| format!("invalid tx: {}", htlc_tx_hex))?;

        if tx_hash.starts_with("error") {
            anyhow::bail!("invalid tx: {}", htlc_tx_hex)
        }

        println!("{}", tx_hash);

        Ok(())
    } 
}
