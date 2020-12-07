use bitcoin_cash::{Opcode::*, ByteArray, Hash160, Integer, Pubkey, Signatory, SignatoryKindOne, SigHashFlags, MAX_SIGNATURE_SIZE, TxPreimage, Script, TxOutput};

pub struct SlpHtlcParams {
    pub secret_hash: Hash160,
    pub seller_pkh: Hash160,
    pub buyer_pkh: Hash160,
    pub timeout: Integer,
}

#[derive(Clone)]
pub enum SlpHtlcSignatory {
    Redeem {
        seller_pk: Pubkey,
        secret: ByteArray,
    },
    Timeout {
        buyer_pk: Pubkey,
    },
}

#[bitcoin_cash::script(
    SlpHtlcInputs,
    Redeem="is_redeem",
    Timeout="!is_redeem",
)]
#[allow(unused_variables)]
pub fn script(
    params: &SlpHtlcParams,
    sig: ByteArray,
    pk: Pubkey,
    #[variant(Redeem)] secret: ByteArray,
    is_redeem: bool,
) {
    OP_IF(is_redeem); {
        let secret_hash = OP_HASH160(secret);
        let expected_hash = params.secret_hash;
        OP_EQUALVERIFY(secret_hash, expected_hash);
        let expected_pkh = params.seller_pkh;
    } OP_ELSE; {
        let timeout = params.timeout;
        OP_CHECKLOCKTIMEVERIFY(timeout);
        OP_DROP(timeout);
        let expected_pkh = params.buyer_pkh;
    } OP_ENDIF;
    OP_OVER(pk, __);
    let pkh = OP_HASH160(pk);
    OP_EQUALVERIFY(expected_pkh, pkh);
    OP_CHECKSIG(sig, pk);
}

impl Signatory for SlpHtlcSignatory {
    type Script=SlpHtlcInputs;
    type Signatures=ByteArray;
    type Kind=SignatoryKindOne;

    fn sig_hash_flags(&self) -> SigHashFlags {
        SigHashFlags::DEFAULT
    }

    fn placeholder_signatures(&self) -> Self::Signatures {
        vec![0; MAX_SIGNATURE_SIZE].into()
    }

    fn build_script(
        &self,
        _tx_preimages: &TxPreimage,
        _estimated_size: Option<usize>,
        sig: ByteArray,
        _lock_script: &Script,
        _tx_outputs: &[TxOutput],
    ) -> Self::Script {
        let sig = sig.concat([self.sig_hash_flags().bits() as u8]);
        match *self {
            SlpHtlcSignatory::Redeem { seller_pk, ref secret } => {
                SlpHtlcInputs::Redeem {
                    sig,
                    pk: seller_pk,
                    secret: secret.clone(),
                    is_redeem: true,
                }
            }
            SlpHtlcSignatory::Timeout { buyer_pk } => {
                SlpHtlcInputs::Timeout {
                    sig,
                    pk: buyer_pk,
                    is_redeem: false,
                }
            }
        }
    }
}
