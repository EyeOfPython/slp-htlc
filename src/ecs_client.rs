use bitcoin_cash::{Address, TxOutpoint, Sha256d, Hashed};
use chttp::{http::StatusCode, prelude::*};

use anyhow::{Context, Result};

pub struct ECSClient<'a> {
    uri: String,
    address_prefix: &'a str,
}

pub struct Utxo {
    pub address: Address<'static>,
    pub value: u64,
    pub outpoint: TxOutpoint,
}

impl<'a> ECSClient<'a> {
    pub fn new(
        uri: String,
        address_prefix: &'a str,
    ) -> Self {
        ECSClient {
            uri,
            address_prefix,
        }
    }

    pub fn createaddress(&self) -> Result<Address> {
        #[derive(serde::Serialize)]
        struct Params {}
        let address_suffix: String = self.ecs_request(
            "getunusedaddress",
            Params {},
        )?;
        let cash_addr = format!("{}:{}", self.address_prefix, address_suffix);
        let address = Address::from_cash_addr(&cash_addr)
            .with_context(|| format!("createnewaddress invalid address generated: {}", cash_addr))?
            .to_owned_address();
        return Ok(address)
    }

    pub fn payto_slp(&self, token_id: &str, amount: &str, destination: &str) -> Result<String> {
        #[derive(serde::Serialize)]
        struct Params<'a> {
            token_id: &'a str,
            amount_slp: &'a str,
            destination_slp: &'a str,
        }
        #[derive(serde::Deserialize)]
        struct Res {
            hex: String,
        }

        let result: Res = self.ecs_request(
            "payto_slp",
            Params {
                token_id,
                amount_slp: amount,
                destination_slp: destination,
            }
        )?;
        return Ok(result.hex)
    }

    pub fn signtransaction(&self, tx_hex: &str) -> Result<String> {
        #[derive(serde::Serialize)]
        struct Params<'a> {
            tx: &'a str,
        }
        #[derive(serde::Deserialize)]
        struct Res {
            hex: String,
        }

        let result: Res = self.ecs_request(
            "signtransaction",
            Params {tx: tx_hex}
        )?;
        return Ok(result.hex)
    }

    pub fn broadcast(&self, tx_hex: &str) -> Result<String> {
        #[derive(serde::Serialize)]
        struct Params<'a> {
            tx: &'a str,
        }

        let result: (bool, String) = self.ecs_request(
            "broadcast",
            Params {tx: tx_hex}
        )?;
        return Ok(result.1)
    }

    pub fn slpvalidate(&self, txid: &str) -> Result<bool> {
        #[derive(serde::Serialize)]
        struct Params<'a> {
            txid: &'a str,
            debug: bool,
            reset: bool,
        }

        let result: Result<String> = self.ecs_request(
            "slpvalidate",
            Params {
                txid,
                debug: true,
                reset: false,
            },
        );
        if result.is_err() {
            println!("retry slpvalidate...");
            return self.slpvalidate(txid);
        }
        let result = result?;
        if result != "Valid" {
            println!("SLP result: {}", result);
        }
        return Ok(result == "Valid")
    }

    pub fn listunspent(&self) -> Result<Vec<Utxo>> {
        #[derive(serde::Serialize)]
        struct Params {}

        #[derive(Debug, serde::Deserialize)]
        pub struct Unspent {
            pub address: String,
            pub value: String,
            pub prevout_n: u32,
            pub prevout_hash: String,
        }

        let result: Vec<Unspent> = self.ecs_request(
            "listunspent",
            Params {}
        )?;

        let mut utxos = Vec::with_capacity(result.len());
        for unspent in result {
            let cash_addr = format!("{}:{}", self.address_prefix, unspent.address);
            let address = Address::from_cash_addr(&cash_addr)
                .with_context(|| format!("listunspent invalid address generated: {}", cash_addr))?
                .to_owned_address();
            let value: f64 = unspent.value.parse()
                .with_context(|| format!("listunspent invalid value: {:?}", unspent.value))?;
            utxos.push(Utxo {
                address,
                value: (value * 100_000_000.0).round() as u64,  // this *should* always be exact
                outpoint: TxOutpoint {
                    tx_hash: Sha256d::from_hex_le(&unspent.prevout_hash)?,
                    vout: unspent.prevout_n,
                },
            })
        }

        return Ok(utxos)
    }

    pub fn gettransaction(&self, txid: &str) -> Result<String> {
        #[derive(serde::Serialize)]
        struct Params<'a> {
            txid: &'a str,
        }

        #[derive(serde::Deserialize)]
        struct Res {
            hex: String,
        }

        let result: Res = self.ecs_request(
            "gettransaction",
            Params { txid },
        )?;
        return Ok(result.hex)
    }

    pub fn getprivatekeys(&self, address: &str) -> Result<[u8; 32]> {
        #[derive(serde::Serialize)]
        struct Params<'a> {
            address: &'a str,
        }
        let result: String = self.ecs_request(
            "getprivatekeys",
            Params { address },
        )?;
        let sk = bitcoin::PrivateKey::from_wif(&result).with_context(|| "getprivatekeys invalid private key")?;
        return Ok(*sk.key.as_ref())
    }

    fn ecs_request<P: serde::Serialize, R: serde::de::DeserializeOwned>(&self, method: &str, params: P) -> Result<R> {
        #[derive(serde::Serialize)]
        struct Req<'a, P> {
            id: u32,
            method: &'a str,
            params: P,
        }

        #[derive(serde::Deserialize)]
        struct Resp<R> {
            result: Option<R>,
            error: Option<ErrorJson>,
        }

        #[derive(serde::Deserialize)]
        #[allow(dead_code)]
        struct ErrorJson {
            code: i32,
            message: String,
        }

        let req = Req {
            id: 0,
            method,
            params,
        };
        let body = serde_json::to_string(&req)
            .with_context(|| format!("{} JSON to_string failed", method))?;
        let mut response = Request::post(&self.uri)
            .body(body.clone())
            .with_context(|| format!("{} body failed", method))?
            .send()
            .with_context(|| format!("{} send failed", method))?;
        if response.status() == StatusCode::OK {
            let response_text = response.text()?;
            let resp: Resp<R> = serde_json::from_str(&response_text)
                .with_context(|| format!("{} invalid json: {}", method, response_text))?;
            if let Some(err) = resp.error {
                anyhow::bail!("{} error: {} (for {})", method, err.message, body)
            }
            return Ok(resp.result.expect("No error but also no result"));
        } else {
            anyhow::bail!("{} invalid response: {}", method, response.text()?);
        }
    }
}
