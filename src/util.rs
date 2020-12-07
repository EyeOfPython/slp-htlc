use bitcoin_cash::*;
use bitcoin_cash_slp::TokenId;

use anyhow::Result;

use crate::ecs_client::ECSClient;

pub fn get_utxo_token_amount(client: &ECSClient, txid: &str, vout: u32) -> Result<(TokenId, u64)> {
    let tx_hex = client.gettransaction(txid)?;
    let raw_tx = hex::decode(&tx_hex)?;
    let (tx, _): (UnhashedTx, _) = UnhashedTx::deser(raw_tx.into())?;
    let slp_ops = tx.outputs[0].script.ops();
    let utxo_amount = if let Op::PushByteArray {array, ..} = &slp_ops[vout as usize + 4].op {
        let mut amount = [0; 8];
        amount.copy_from_slice(&array);
        u64::from_be_bytes(amount)
    } else {
        unreachable!()
    };
    let token_id = if let Op::PushByteArray {array, ..} = &slp_ops[4].op {
        TokenId::from_slice(array)?
    } else {
        unreachable!()
    };
    Ok((token_id, utxo_amount))
}

pub fn add_gas_inputs<'b>(client: &ECSClient, ecc: &impl ECC, mut tx_builder: TxBuilder<'b>) -> Result<(UnsignedTx<'b>, Vec<(InputReference<P2PKHSignatory>, [u8; 32])>)> {
    let mut utxos = client.listunspent()?;
    let mut gas_inputs = Vec::new();
    let fee_rate = 1;
    let unsigned_tx = loop {
        if utxos.len() == 0 {
            anyhow::bail!("Insufficient funds (not enough 'gas' in BCH)");
        }
        let next_utxo = utxos.remove(0);
        let utxo_sk = client.getprivatekeys(&next_utxo.address.cash_addr())?;
        let utxo_pk = ecc.derive_pubkey(&utxo_sk)?;
        let gas_ref = tx_builder.add_input(
            UnsignedTxInput {
                prev_out: next_utxo.outpoint.clone(),
                sequence: 0xffff_ffff,
                value: next_utxo.value,
            },
            next_utxo.address.p2pkh_script()?,
            P2PKHSignatory {
                pubkey: utxo_pk,
                sig_hash_flags: SigHashFlags::DEFAULT,
            },
        );
        gas_inputs.push((gas_ref, utxo_sk));
        let known_output_sum = tx_builder.known_output_sum();
        let input_sum = tx_builder.input_sum();
        if known_output_sum > input_sum {
            continue;
        }
        let leftover = input_sum - known_output_sum;
        let unsigned_tx = tx_builder.build()?;
        if unsigned_tx.estimated_size() * fee_rate > leftover as usize {
            tx_builder = unsigned_tx.into_tx_builder();
            continue;
        }
        break unsigned_tx;
    };
    Ok((unsigned_tx, gas_inputs))
}
