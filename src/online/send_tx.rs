use crate::*;
use bitcoincore_rpc::RpcApi;
use log::{debug, info};

#[derive(structopt::StructOpt, Debug)]
pub struct SendTxOptions {
    /// filename containing the PSBT
    #[structopt(long = "psbt")]
    pub psbts: Vec<PathBuf>,

    /// broadcast transaction through the node, by default it is not broadcasted
    #[structopt(long)]
    pub broadcast: bool,
}

impl SendTxOptions {
    fn validate(&self) -> Result<()> {
        if self.psbts.is_empty() {
            return firma::err("At least one psbt is mandatory");
        }
        Ok(())
    }
}

impl Wallet {
    pub fn send_tx(&self, opt: &SendTxOptions) -> Result<()> {
        opt.validate()?;
        let mut psbts = vec![];
        for psbt_file in opt.psbts.iter() {
            let json = read_psbt(psbt_file)?;
            psbts.push(json.signed_psbt.expect("signed_psbt not found"));
        }
        let combined = self.client.combine_psbt(&psbts)?;
        debug!("combined {:?}", combined);

        let finalized = self.client.finalize_psbt(&combined, Some(true))?;
        debug!("finalized {:?}", finalized);

        let hex = finalized.hex.ok_or_else(fn_err("hex is empty"))?;

        if opt.broadcast {
            let hash = self.client.send_raw_transaction(hex)?;
            info!("{:?}", hash);
        } else {
            info!("{}", hex);
        }

        Ok(())
    }
}
