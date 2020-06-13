use crate::*;
use bitcoin::blockdata::opcodes;
use bitcoin::blockdata::script::Instruction::PushBytes;
use bitcoin::consensus::{deserialize, serialize};
use bitcoin::util::key;
use bitcoin::{Script, Transaction};
use log::{LevelFilter, Metadata, Record};
use std::fs::OpenOptions;
use std::io::BufWriter;
use std::io::Write;

pub mod cmd;
pub mod error;
pub mod file;
pub mod json;
pub mod list;
pub mod mnemonic;
pub mod qr;

static LOGGER: SimpleLogger = SimpleLogger;

pub struct SimpleLogger;

impl log::Log for SimpleLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= log::max_level()
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let file = OpenOptions::new()
                .create(true)
                .append(true)
                .open("firma.log")
                .expect("can't open log");
            let mut stream = BufWriter::new(file);
            stream
                .write_all(format!("{} - {}\n", record.level(), record.args()).as_bytes())
                .expect("can't write log");
        }
    }

    fn flush(&self) {}
}

pub fn init_logger() {
    log::set_logger(&LOGGER)
        .map(|()| log::set_max_level(LevelFilter::Debug))
        .expect("cannot initialize logging");
}

impl From<PrivateMasterKey> for PublicMasterKey {
    fn from(private: PrivateMasterKey) -> Self {
        PublicMasterKey { xpub: private.xpub }
    }
}

pub fn psbt_from_base64(s: &str) -> Result<(Vec<u8>, PSBT)> {
    let bytes = base64::decode(s)?;
    let psbt = deserialize(&bytes)?;
    Ok((bytes, psbt))
}

pub fn psbt_to_base64(psbt: &PSBT) -> (Vec<u8>, String) {
    let bytes = serialize(psbt);
    let string = base64::encode(&bytes);
    (bytes, string)
}

pub fn estimate_weight(psbt: &PSBT) -> Result<usize> {
    let unsigned_weight = psbt.global.unsigned_tx.get_weight();
    let mut spending_weight = 0usize;

    for input in psbt.inputs.iter() {
        let (script, factor) = match (&input.redeem_script, &input.witness_script) {
            (Some(redeem_script), None) => (redeem_script, 4),
            (_, Some(witness_script)) => (witness_script, 1), // factor=1 for segwit discount
            _ => return Err("both redeem and witness script are None".into()),
        };
        //TODO signature are less in NofM where N<M
        let current = script.len() + expected_signatures(script)? * 72; // using 72 as average signature size
        spending_weight += current * factor;
    }

    Ok(unsigned_weight + spending_weight)
}

fn expected_signatures(script: &Script) -> Result<usize> {
    let bytes = script.as_bytes();
    Ok(
        if bytes.last().ok_or_else(|| Error::ScriptEmpty)?
            == &opcodes::all::OP_CHECKMULTISIG.into_u8()
        {
            read_pushnum(bytes[0])
                .map(|el| el as usize)
                .unwrap_or(0usize)
        } else {
            extract_pub_keys(script)?.len()
        },
    )
}

fn read_pushnum(value: u8) -> Option<u8> {
    if value >= opcodes::all::OP_PUSHNUM_1.into_u8()
        && value <= opcodes::all::OP_PUSHNUM_16.into_u8()
    {
        Some(value - opcodes::all::OP_PUSHNUM_1.into_u8() + 1)
    } else {
        None
    }
}

pub fn extract_pub_keys(script: &Script) -> Result<Vec<key::PublicKey>> {
    let mut result = vec![];
    for instruct in script.iter(false) {
        if let PushBytes(a) = instruct {
            if a.len() == 33 {
                result.push(key::PublicKey::from_slice(&a)?);
            }
        }
    }
    Ok(result)
}

pub fn unwrap_as_json(result: Result<serde_json::Value>) -> serde_json::Value {
    result.unwrap_or_else(|e| e.to_json())
}

pub fn map_json_error(result: Result<serde_json::Value>) -> Result<serde_json::Value> {
    match result {
        Ok(value) => match value.get("error") {
            Some(serde_json::Value::String(e)) => Err(Error::Generic(e.to_string())),
            _ => Ok(value),
        },
        Err(e) => Err(Error::Generic(e.to_string())),
    }
}

pub fn strip_witness(tx: &Transaction) -> Transaction {
    let cloned_tx = Transaction {
        version: tx.version,
        lock_time: tx.lock_time,
        input: tx
            .input
            .iter()
            .map(|txin| bitcoin::TxIn {
                witness: vec![],
                ..txin.clone()
            })
            .collect(),
        output: tx.output.clone(),
    };
    cloned_tx
}

#[cfg(test)]
mod tests {
    use crate::strip_witness;
    use bitcoin::consensus::deserialize;
    use bitcoin::Transaction;

    #[test]
    fn test_strip() {
        let segwit_tx = "020000000001019c644affd9c62cef3a13c4d2facc4284bcce3f1769d4aeda062413ece120ffc80100000000ffffffff029b660900000000001600147a5d9c9672cb9c788c2b7f217a8b35af6e3f7e8bdee60300000000001976a914228e6b93d66a870fabb41dd064dedbd14804431388ac024730440220453ca5656c155e63bea0af0e83d59ea7097c3cc5bfef5abade3c7d49435fcc3a0220404c3d469fbcee2ace5bf5963440eb78ca63c40c2fe80547026a48009ed0009e01210336d86e06d33b04ed236d280590f1a6d0c6eb7f703b7fe78cc1d71122d0c4f9be00000000";
        let segwit_tx: Transaction = deserialize(&hex::decode(segwit_tx).unwrap()).unwrap();
        let stripped = strip_witness(&segwit_tx);
        assert_eq!(segwit_tx.txid(), stripped.txid());
        assert!(stripped.get_weight() < segwit_tx.get_weight());
    }
}
