use crate::list::ListOptions;
use crate::offline::descriptor::{derive_address, DeriveAddressOpts};
use crate::*;
use bitcoin::consensus::serialize;
use bitcoin::util::bip32::{ChildNumber, DerivationPath, Fingerprint};
use bitcoin::util::key;
use bitcoin::{Address, Amount, Network, OutPoint, Script, SignedAmount, TxOut};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::PathBuf;
use structopt::StructOpt;

type HDKeypaths = BTreeMap<key::PublicKey, (Fingerprint, DerivationPath)>;

/// Print details regarding a Partially Signed Bitcoin Transaction (PSBT) given as parameter.
/// A `psbt_file` or a `psbt_base` should be specified.
#[derive(StructOpt, Debug, Serialize, Deserialize)]
#[structopt(name = "firma")]
pub struct PrintOptions {
    /// PSBT json file
    #[structopt(long)]
    pub psbt_file: Option<PathBuf>,

    /// PSBT as base64 string
    #[structopt(long)]
    pub psbt_base64: Option<String>,

    /// Return wallets only if wallet signature file is present and signature verifies
    #[structopt(long)]
    pub verify_wallets_signatures: bool,
}

pub fn start(datadir: &str, network: Network, opt: &PrintOptions) -> Result<PsbtPrettyPrint> {
    let psbt = match (&opt.psbt_file, &opt.psbt_base64) {
        (Some(path), None) => read_psbt(path)?,
        (None, Some(base64)) => psbt_from_base64(base64)?.1,
        (None, None) => return Err("`psbt_file` or `psbt_base64` must be set".into()),
        (Some(_), Some(_)) => {
            return Err("`psbt_file` and `psbt_base64` cannot be both specified".into())
        }
    };
    let kind = Kind::Wallet;
    let opt = ListOptions {
        kind,
        verify_wallets_signatures: opt.verify_wallets_signatures,
        encryption_keys: vec![],
    };
    let result = common::list::list(datadir, network, &opt)?;
    let wallets: Vec<WalletJson> = result.wallets.iter().map(|w| w.wallet.clone()).collect();
    let output = pretty_print(&psbt, network, &wallets)?;
    Ok(output)
}

pub fn pretty_print(
    psbt: &PSBT,
    network: Network,
    wallets: &[WalletJson],
) -> Result<PsbtPrettyPrint> {
    let mut result = PsbtPrettyPrint::default();
    let mut previous_outputs: Vec<TxOut> = vec![];
    let mut output_values: Vec<u64> = vec![];
    let tx = &psbt.global.unsigned_tx;
    let vouts: Vec<OutPoint> = tx.input.iter().map(|el| el.previous_output).collect();
    for (i, input) in psbt.inputs.iter().enumerate() {
        let previous_output = match (&input.non_witness_utxo, &input.witness_utxo) {
            (_, Some(val)) => val,
            (Some(prev_tx), None) => {
                let outpoint = *vouts.get(i).ok_or(Error::MissingOutpoint)?;
                assert_eq!(prev_tx.txid(), outpoint.txid);
                prev_tx
                    .output
                    .get(outpoint.vout as usize)
                    .ok_or(Error::MissingTxout)?
            }
            _ => return Err("witness_utxo and non_witness_utxo are both None".into()),
        };
        previous_outputs.push(previous_output.clone());
    }
    let input_values: Vec<u64> = previous_outputs.iter().map(|o| o.value).collect();
    let mut balances = HashMap::new();

    for (i, input) in tx.input.iter().enumerate() {
        let addr = Address::from_script(&previous_outputs[i].script_pubkey, network)
            .ok_or(Error::NonDefaultScript)?;
        let keypaths = &psbt.inputs[i].hd_keypaths;
        let signatures: HashSet<Fingerprint> = psbt.inputs[i]
            .partial_sigs
            .iter()
            .filter_map(|(k, _)| keypaths.get(k).map(|v| v.0))
            .collect();
        let wallet_if_any = wallet_with_path(keypaths, &wallets, &addr);
        if let Some((wallet, _)) = &wallet_if_any {
            *balances.entry(wallet.clone()).or_insert(0i64) -= previous_outputs[i].value as i64
        }
        let txin = json::TxIn {
            outpoint: input.previous_output.to_string(),
            signatures,
            common: TxCommonInOut {
                value: Amount::from_sat(previous_outputs[i].value).to_string(),
                wallet_with_path: wallet_if_any.map(|(w, p)| format!("[{}]{}", w, p)),
            },
        };
        result.inputs.push(txin);
    }

    for (i, output) in tx.output.iter().enumerate() {
        let addr =
            Address::from_script(&output.script_pubkey, network).ok_or(Error::NonDefaultScript)?;
        let keypaths = &psbt.outputs[i].hd_keypaths;
        let wallet_if_any = wallet_with_path(keypaths, &wallets, &addr);
        if let Some((wallet, _)) = &wallet_if_any {
            *balances.entry(wallet.clone()).or_insert(0i64) += output.value as i64
        }
        let txout = json::TxOut {
            address: addr.to_string(),
            common: TxCommonInOut {
                value: Amount::from_sat(output.value).to_string(),
                wallet_with_path: wallet_if_any.map(|(w, p)| format!("[{}]{}", w, p)),
            },
        };
        result.outputs.push(txout);
        output_values.push(output.value);
    }
    let balances_vec: Vec<String> = balances
        .iter()
        .map(|(k, v)| format!("{}: {}", k, SignedAmount::from_sat(*v).to_string()))
        .collect();
    result.balances = balances_vec.join("\n");

    // Privacy analysis
    // Detect different script types in the outputs
    let mut script_types = HashSet::new();
    for o in tx.output.iter() {
        script_types.insert(script_type(&o.script_pubkey));
    }
    if script_types.len() > 1 {
        result.info.push("Privacy: outputs have different script types https://en.bitcoin.it/wiki/Privacy#Sending_to_a_different_script_type".to_string());
    }

    // Detect rounded amounts
    let divs: Vec<u8> = tx
        .output
        .iter()
        .map(|o| biggest_dividing_pow(o.value))
        .collect();
    if let (Some(max), Some(min)) = (divs.iter().max(), divs.iter().min()) {
        if max - min >= 3 {
            result.info.push("Privacy: outputs have different precision https://en.bitcoin.it/wiki/Privacy#Round_numbers".to_string());
        }
    }

    // Detect unnecessary input heuristic
    if previous_outputs.len() > 1 {
        if let Some(smallest_input) = input_values.iter().min() {
            if output_values.iter().any(|value| value < smallest_input) {
                result.info.push("Privacy: smallest output is smaller then smallest input https://en.bitcoin.it/wiki/Privacy#Unnecessary_input_heuristic".to_string());
            }
        }
    }

    // Detect script reuse
    let input_scripts: HashSet<Script> = previous_outputs
        .iter()
        .map(|o| o.script_pubkey.clone())
        .collect();
    if tx
        .output
        .iter()
        .any(|o| input_scripts.contains(&o.script_pubkey))
    {
        result.info.push(
            "Privacy: address reuse https://en.bitcoin.it/wiki/Privacy#Address_reuse".to_string(),
        );
    }

    let fee = input_values.iter().sum::<u64>() - output_values.iter().sum::<u64>();
    let tx_vbytes = tx.get_weight() / 4;
    let estimated_tx_vbytes = estimate_weight(psbt).ok().map(|e| e / 4);
    let estimated_fee_rate = estimated_tx_vbytes.map(|e| fee as f64 / e as f64);

    result.size = Size {
        estimated: estimated_tx_vbytes,
        unsigned: tx_vbytes,
        psbt: serialize(psbt).len(),
    };
    result.fee = Fee {
        absolute: fee,
        absolute_fmt: Amount::from_sat(fee).to_string(),
        rate: estimated_fee_rate,
    };

    Ok(result)
}

fn biggest_dividing_pow(num: u64) -> u8 {
    let mut start = 10u64;
    let mut count = 0u8;
    loop {
        if num % start != 0 {
            return count;
        }
        start *= 10;
        count += 1;
    }
}

const SCRIPT_TYPE_FN: [fn(&Script) -> bool; 5] = [
    Script::is_p2pk,
    Script::is_p2pkh,
    Script::is_p2sh,
    Script::is_v0_p2wpkh,
    Script::is_v0_p2wsh,
];
fn script_type(script: &Script) -> Option<usize> {
    SCRIPT_TYPE_FN.iter().position(|f| f(script))
}

/// returns a wallet name and a derivation iif the address parameter is the same as the one derived from the wallet
fn wallet_with_path(
    hd_keypaths: &HDKeypaths,
    wallets: &[WalletJson],
    address: &Address,
) -> Option<(String, DerivationPath)> {
    for wallet in wallets {
        for (_, (finger, path)) in hd_keypaths.iter() {
            if wallet.fingerprints.contains(finger) {
                let path_vec: Vec<ChildNumber> = path.clone().into();
                if let ChildNumber::Normal { index } = path_vec.first()? {
                    let descriptor = match index {
                        0 => &wallet.descriptor,
                        _ => return None,
                    };
                    if let ChildNumber::Normal { index } = path_vec.last()? {
                        let opts = DeriveAddressOpts {
                            descriptor: descriptor.to_string(),
                            index: *index,
                        };
                        if let Ok(derived) = derive_address(address.network, &opts) {
                            if &derived.address == address {
                                return Some((wallet.name.clone(), path.clone()));
                            }
                        }
                    }
                };
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use crate::offline::print::{biggest_dividing_pow, script_type};

    #[test]
    fn test_biggest_dividing_pow() {
        assert_eq!(biggest_dividing_pow(3), 0);
        assert_eq!(biggest_dividing_pow(10), 1);
        assert_eq!(biggest_dividing_pow(11), 0);
        assert_eq!(biggest_dividing_pow(110), 1);
        assert_eq!(biggest_dividing_pow(1100), 2);
        assert_eq!(biggest_dividing_pow(1100030), 1);
    }

    #[test]
    fn test_script_type() {
        macro_rules! hex_script (($s:expr) => (bitcoin::blockdata::script::Script::from(::hex::decode($s).unwrap())));

        let s =
            hex_script!("21021aeaf2f8638a129a3156fbe7e5ef635226b0bafd495ff03afe2c843d7e3a4b51ac");
        assert_eq!(script_type(&s), Some(0usize));

        let s = hex_script!("76a91402306a7c23f3e8010de41e9e591348bb83f11daa88ac");
        assert_eq!(script_type(&s), Some(1usize));

        let s = hex_script!("a914acc91e6fef5c7f24e5c8b3f11a664aa8f1352ffd87");
        assert_eq!(script_type(&s), Some(2usize));

        let s = hex_script!("00140c3e2a4e0911aac188fe1cba6ef3d808326e6d0a");
        assert_eq!(script_type(&s), Some(3usize));

        let s = hex_script!("00201775ead41acefa14d2d534d6272da610cc35855d0de4cab0f5c1a3f894921989");
        assert_eq!(script_type(&s), Some(4usize));
    }
}
