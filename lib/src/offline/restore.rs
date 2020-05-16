use crate::mnemonic::Mnemonic;
use crate::Result;
use crate::{save_keys, MasterKeyOutput, PrivateMasterKey};
use bitcoin::util::bip32::ExtendedPrivKey;
use bitcoin::Network;
use log::debug;
use serde::{Deserialize, Serialize};
use std::io;
use std::str::FromStr;
use structopt::StructOpt;

/// Restore a master key from the secret component
#[derive(StructOpt, Debug, Deserialize, Serialize)]
#[structopt(name = "restore")]
pub struct RestoreOptions {
    /// Name of the key
    #[structopt(short, long)]
    key_name: String,

    /// Kind of the secret material
    #[structopt(short, long)]
    nature: Nature,

    /// QR code max version to use (max size)
    #[structopt(long, default_value = "14")]
    pub qr_version: i16,

    /// value of the secret component, could be xprv or seed in hex or bech32
    value: String,
}

#[derive(Debug, Deserialize, Serialize)]
enum Nature {
    Xprv,
    Mnemonic,
}

impl FromStr for Nature {
    type Err = io::Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "xprv" => Ok(Nature::Xprv),
            "mnemonic" => Ok(Nature::Mnemonic),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("({}) valid values are: xprv, hex-seed, bech32-seed", s),
            )),
        }
    }
}

pub fn start(datadir: &str, network: Network, opt: &RestoreOptions) -> Result<MasterKeyOutput> {
    debug!("restore {:?}", &opt);
    let master_key = match opt.nature {
        Nature::Xprv => {
            PrivateMasterKey::from_xprv(ExtendedPrivKey::from_str(&opt.value)?, &opt.key_name)
        }
        Nature::Mnemonic => {
            let mnemonic = Mnemonic::from_str(&opt.value)?;
            PrivateMasterKey::new(network, &mnemonic, &opt.key_name)?
        }
    };

    let output = save_keys(datadir, network, &opt.key_name, master_key, opt.qr_version)?;
    Ok(output)
}

#[cfg(test)]
mod tests {
    use crate::offline::random::RandomOptions;
    use crate::offline::restore::{Nature, RestoreOptions};
    use bitcoin::Network;
    use tempdir::TempDir;

    #[test]
    fn test_restore() {
        let temp_dir = TempDir::new("test_restore").unwrap();
        let temp_dir_str = format!("{}/", temp_dir.path().display());
        let key_name_random = "test_restore_random".to_string();
        let rand_opts = RandomOptions::new(key_name_random);
        let name_counter = 0;

        let key_orig =
            crate::offline::random::create_key(&temp_dir_str, Network::Testnet, &rand_opts)
                .unwrap();

        let (key_name, name_counter) = (format!("{}", name_counter), name_counter + 1);
        let restore_opts = RestoreOptions {
            key_name,
            nature: Nature::Xprv,
            value: key_orig.key.xprv.to_string(),
            qr_version: 14,
        };
        let key_restored =
            crate::offline::restore::start(&temp_dir_str, Network::Testnet, &restore_opts).unwrap();
        assert_eq!(key_orig.key.xprv, key_restored.key.xprv);
        assert_eq!(key_orig.key.xpub, key_restored.key.xpub);

        let (key_name, name_counter) = (format!("{}", name_counter), name_counter + 1);
        let restore_opts = RestoreOptions {
            key_name,
            nature: Nature::Mnemonic,
            value: key_orig.key.mnemonic.unwrap().to_string(),
            qr_version: 14,
        };
        let key_restored =
            crate::offline::restore::start(&temp_dir_str, Network::Testnet, &restore_opts).unwrap();
        assert_eq!(key_orig.key.xprv, key_restored.key.xprv);
        assert_eq!(key_orig.key.xpub, key_restored.key.xpub);

        // TODO add restore mnemonic

        let (key_name, name_counter) = (format!("{}", name_counter), name_counter + 1);
        let restore_opts = RestoreOptions {
            key_name,
            nature: Nature::Xprv,
            value: "X".to_string(),
            qr_version: 14,
        };
        let result = crate::offline::restore::start(&temp_dir_str, Network::Testnet, &restore_opts);
        assert!(result.is_err());

        let (key_name, _name_counter) = (format!("{}", name_counter), name_counter + 1);
        let restore_opts = RestoreOptions {
            key_name,
            nature: Nature::Xprv,
            value: key_orig.key.xpub.to_string(),
            qr_version: 14,
        };
        let result = crate::offline::restore::start(&temp_dir_str, Network::Testnet, &restore_opts);
        assert!(result.is_err());
    }
}
