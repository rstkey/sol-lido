use clap::Clap;
use serde::Deserialize;
use serde_json::Value;
use solana_sdk::pubkey::{ParsePubkeyError, Pubkey};
use std::{path::PathBuf, str::FromStr};

pub fn get_option_from_config<T: FromStr>(
    name: &'static str,
    config_file: Option<&ConfigFile>,
) -> Option<T> {
    let config_file = config_file?;
    let value = config_file.values.get(name)?;
    if let Value::String(str_value) = value {
        match T::from_str(str_value) {
            Err(_) => {
                eprintln!("Could not convert {} from string", str_value);
                std::process::exit(1);
            }
            Ok(pubkey) => Some(pubkey),
        }
    } else {
        // TODO: Support numbers
        None
    }
}
/// Generates a struct that derives `Clap` for usage with a config file.
///
/// This macro avoids code repetition by implementing a function that sweeps
/// every field from the struct and checks if it is set either by argument or by
/// a config file. If the field is set by neither, it will print all the
/// missing fields.
/// It will also implement getters that unwrap every field of the struct,
/// this is necessary to use optional arguments in clap, that will be filled
/// in case they were defined in the configuration file.

/// Optionally, a default value can be passed for the structure with `=> <default>`,
/// If the value is neither passed by argument or by config file, it will be set
/// as `default`.

/// The arguments expected are the same as the names defined in the macro
/// substituting all '_' with '-', in case of the config file, the names are the
/// same as defined in the struct.

/// Example:
/// ```
/// cli_opt_struct! {
///     FooOpts {
///         #[clap(long, value_name = "address")]
///         foo_arg: Pubkey,
///         def_arg: i32 => 3 // argument is 3 by default
///     }
/// }
/// ```
/// This generates the struct:
/// ```
/// struct FooOpts {
///     #[clap(long, value_name = "address")]
///     foo_arg: Option<Pubkey>,
///     def_arg: Option<i32>, // If not present in config, the value will be 3 by default.
/// }
///
/// impl FooOpts {
///     pub fn merge_with_config(&mut self, config_file: &Option<ConfigFile>);
/// }
/// ```
/// When `merge_with_config(config_file)` is called, if the `foo` field has a
/// value (set by passing `--foo-arg <pubkey>`) it does nothing, otherwise,
/// search `config_file` for the key 'foo_arg' and sets the field accordingly.
/// The type must implement the `FromStr` trait.
/// In the example, `def_arg` will have value 3 if not present in the config file.

macro_rules! cli_opt_struct {
    {
        // Struct name
        $name:ident {
            $(
                // Foward the properties
                $(#[$attr:meta])*
                // Field name and type, specify default value
                $field:ident : $type:ty $(=> $default:expr)?
            ),*
            $(,)?
        }
    } => {
        #[derive(Debug, Clap)]
        pub struct $name {
            $(
                $(#[$attr])*
                $field: Option<$type>,
            )*
        }

        impl $name {
            /// Merges the struct with a config file.
            /// Fails if a field is not present (None) in the struct *and* not
            /// present in the config file. When failing, prints all the missing
            /// fields.
            #[allow(dead_code)]
            pub fn merge_with_config(&mut self, config_file: Option<&ConfigFile>) {
                let mut failed = false;
                $(
                    let from_cli = self.$field.take();
                    let str_field = stringify!($field);
                    let from_config = get_option_from_config(str_field, config_file);

                    #[allow(unused_mut, unused_assignments)]
                    let mut default = None;
                    $(default = Some($default);)?
                    // Sets the field with the argument or the config field.
                    self.$field = from_cli.or(from_config).or(default);
                    if self.$field.is_none() {
                        failed = true;
                        eprintln!("Expected --{} to be provided on the command line, or set in config file with key {}.", str_field.replace("_", "-"), str_field);
                    }
                )*
                if failed {
                    std::process::exit(1);
                }
            }

            $(
                // Implement a getter for every field in the struct
                #[allow(dead_code)]
                pub fn $field(&self) -> &$type {
                    self.$field.as_ref().unwrap()
                }
            )*
        }
    }
}

/// Type to represent a vector of `Pubkey`.
// TODO(#218) Accept an array in the json config file.
#[derive(Debug, Clone)]
pub struct PubkeyVec(pub Vec<Pubkey>);
/// Constructs a `PubkeyVec` from a string by splitting the string by ',' and
/// constructing a Pubkey for each of the tokens
impl FromStr for PubkeyVec {
    type Err = ParsePubkeyError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let pubkeys = s
            .split(',')
            .map(|key| Pubkey::from_str(key))
            .collect::<Result<Vec<Pubkey>, Self::Err>>()?;
        Ok(PubkeyVec(pubkeys))
    }
}

#[derive(Debug, Deserialize)]
pub struct ConfigFile {
    pub values: Value,
}

pub fn read_config(config_path: PathBuf) -> ConfigFile {
    let file_content = std::fs::read(config_path).expect("Failed to open config file.");
    let values: Value = serde_json::from_slice(&file_content).expect("Error while reading config.");
    ConfigFile { values }
}

#[derive(Copy, Clone, Debug)]
pub enum OutputMode {
    /// Output human-readable text to stdout.
    Text,

    /// Output machine-readable json to stdout.
    Json,
}

impl FromStr for OutputMode {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<OutputMode, &'static str> {
        match s {
            "text" => Ok(OutputMode::Text),
            "json" => Ok(OutputMode::Json),
            _ => Err("Invalid output mode, expected 'text' or 'json'."),
        }
    }
}

cli_opt_struct! {
    CreateSolidoOpts {
        /// Address of the Solido program
        #[clap(long, value_name = "address")]
        solido_program_id: Pubkey,

        /// The maximum number of validators that this Solido instance will support.
        #[clap(long, value_name = "int")]
        max_validators: u32,

        /// The maximum number of maintainers that this Solido instance will support.
        #[clap(long, value_name = "int")]
        max_maintainers: u32,

        /// Treasury fee share of the rewards.
        ///
        /// A fraction T / (T + V + D + A) of the SOL rewards is paid to the
        /// treasury in the form of stSOL.
        ///
        /// * T: Treasury fee share
        /// * V: Validation fee share
        /// * D: Developer fee share
        /// * A: stSOL appreciation share
        #[clap(long, value_name = "int")]
        treasury_fee_share: u32,

        /// Validation fee share of the rewards.
        ///
        /// A fraction V / (T + V + D + A) of the SOL rewards is divided evenly
        /// among all validators, and they can claim this in the form of stSOL.
        ///
        /// * T: Treasury fee share
        /// * V: Validation fee share
        /// * D: Developer fee share
        /// * A: stSOL appreciation share
        #[clap(long, value_name = "int")]
        validation_fee_share: u32,

        /// Developer fee share of the rewards.
        ///
        /// A fraction D / (T + V + D + A) of the SOL rewards is paid to the
        /// developer in the form of stSOL.
        ///
        /// * T: Treasury fee share
        /// * V: Validation fee share
        /// * D: Developer fee share
        /// * A: stSOL appreciation share
        #[clap(long, value_name = "int")]
        developer_fee_share: u32,

        /// Share of the rewards that goes to stSOL appreciation.
        ///
        /// A fraction A / (T + V + D + A) of SOL the rewards go towards value
        /// increase of stSOL. No stSOL is minted for these SOL rewards, which
        /// means that one stSOL is now a share of a larger pool.
        ///
        /// * T: Treasury fee share
        /// * V: Validation fee share
        /// * D: Developer fee share
        /// * A: stSOL appreciation share
        #[clap(long, value_name = "int")]
        st_sol_appreciation_share: u32,

        /// Account who will own the stSOL SPL token account that receives treasury fees.
        #[clap(long, value_name = "address")]
        treasury_account_owner: Pubkey,

        /// Account who will own the stSOL SPL token account that receives the developer fees.
        #[clap(long, value_name = "address")]
        developer_account_owner: Pubkey,

        /// Used to compute Solido's manager. Multisig instance.
        #[clap(long, value_name = "address")]
        multisig_address: Pubkey,

        /// Address of the Multisig program.
        #[clap(long)]
        multisig_program_id: Pubkey,
    }
}

cli_opt_struct! {
    AddValidatorOpts {
        /// Address of the Solido program.
        #[clap(long, value_name = "address")]
        solido_program_id: Pubkey,
        /// Account that stores the data for this Solido instance.
        #[clap(long, value_name = "address")]
        solido_address: Pubkey,

        /// Address of the validator vote account.
        #[clap(long, value_name = "address")]
        validator_vote_account: Pubkey,

        /// Validator stSol token account.
        #[clap(long, value_name = "address")]
        validator_fee_account: Pubkey,

        /// Multisig instance.
        #[clap(long, value_name = "address")]
        multisig_address: Pubkey,

        /// Address of the Multisig program.
        #[clap(long)]
        multisig_program_id: Pubkey,
    }
}

cli_opt_struct! {
    AddRemoveMaintainerOpts {
        /// Address of the Solido program.
        #[clap(long, value_name = "address")]
        solido_program_id: Pubkey,
        /// Account that stores the data for this Solido instance.
        #[clap(long, value_name = "address")]
        solido_address: Pubkey,

        /// Maintainer to add or remove.
        #[clap(long, value_name = "address")]
        maintainer_address: Pubkey,

        /// Multisig instance.
        #[clap(long, value_name = "address")]
        multisig_address: Pubkey,

        /// Address of the Multisig program.
        #[clap(long)]
        multisig_program_id: Pubkey,
    }
}

cli_opt_struct! {
     ShowSolidoOpts {
        /// The solido instance to show
        #[clap(long, value_name = "address")]
        solido_address: Pubkey,
        /// Address of the Solido program.
        #[clap(long, value_name = "address")]
        solido_program_id: Pubkey,
    }
}

cli_opt_struct! {
    PerformMaintenanceOpts {
        /// Address of the Solido program.
        #[clap(long, value_name = "address")]
        solido_program_id: Pubkey,

        /// Account that stores the data for this Solido instance.
        #[clap(long, value_name = "address")]
        solido_address: Pubkey,
    }
}

// Multisig opts

cli_opt_struct! {
    CreateMultisigOpts {
        /// How many signatures are needed to approve a transaction.
        #[clap(long)]
        threshold: u64,

        /// The public keys of the multisig owners, who can sign transactions.
        #[clap(long = "owner")]
        owners: PubkeyVec,

        /// Address of the Multisig program.
        #[clap(long)]
        multisig_program_id: Pubkey,
    }
}

impl CreateMultisigOpts {
    /// Perform a few basic checks to rule out nonsensical multisig settings.
    ///
    /// Exits if validation fails.
    pub fn validate_or_exit(&self) {
        if *self.threshold() > self.owners().0.len() as u64 {
            println!("Threshold must be at most the number of owners.");
            std::process::exit(1);
        }
        if *self.threshold() == 0 {
            println!("Threshold must be at least 1.");
            std::process::exit(1);
        }
    }
}

cli_opt_struct! {
ProposeUpgradeOpts {
        /// The multisig account whose owners should vote for this proposal.
        #[clap(long, value_name = "address")]
        multisig_address: Pubkey,

        /// The program id of the program to upgrade.
        #[clap(long, value_name = "address")]
        program_address: Pubkey,

        /// The address that holds the new program data.
        #[clap(long, value_name = "address")]
        buffer_address: Pubkey,

        /// Account that will receive leftover funds from the buffer account.
        #[clap(long, value_name = "address")]
        spill_address: Pubkey,

        /// Address of the Multisig program.
        #[clap(long)]
        multisig_program_id: Pubkey,
    }
}

cli_opt_struct! {
ShowMultisigOpts {
        /// The multisig account to display.
        #[clap(long, value_name = "address")]
        multisig_address: Pubkey,

        /// Address of the Multisig program.
        #[clap(long)]
        multisig_program_id: Pubkey,
    }
}

cli_opt_struct! {
    ShowTransactionOpts {
        /// The transaction to display.
        #[clap(long, value_name = "address")]
        transaction_address: Pubkey,

        /// The transaction to display.
        #[clap(long, value_name = "address")]
        solido_program_id: Pubkey,

        /// Address of the Multisig program.
        #[clap(long)]
        multisig_program_id: Pubkey,
    }
}

cli_opt_struct! {
    ApproveOpts {
        // TODO: Can be omitted, we can obtain it from the transaction account.
        /// The multisig account whose owners should vote for this proposal.
        #[clap(long, value_name = "address")]
        multisig_address: Pubkey,

        /// The transaction to approve.
        #[clap(long, value_name = "address")]
        transaction_address: Pubkey,

        /// Address of the Multisig program.
        #[clap(long)]
        multisig_program_id: Pubkey,
    }
}

cli_opt_struct! {
    ExecuteTransactionOpts {
        // TODO: Can be omitted, we can obtain it from the transaction account.
        /// The multisig account whose owners approved this transaction.
        #[clap(long, value_name = "address")]
        multisig_address: Pubkey,

        /// The transaction to execute.
        #[clap(long, value_name = "address")]
        transaction_address: Pubkey,

        /// Address of the Multisig program.
        #[clap(long)]
        multisig_program_id: Pubkey,
    }
}

cli_opt_struct! {
    ProposeChangeMultisigOpts {
        /// The multisig account to modify.
        #[clap(long)]
        multisig_address: Pubkey,

        // The fields below are the same as for `CreateMultisigOpts`, but we can't
        // just embed a `CreateMultisigOpts`, because Clap does not support that.
        /// How many signatures are needed to approve a transaction.
        #[clap(long)]
        threshold: u64,

        /// The public keys of the multisig owners, who can sign transactions.
        #[clap(long = "owner")]
        owners: PubkeyVec,

        /// Address of the Multisig program.
        #[clap(long)]
        multisig_program_id: Pubkey,
    }
}

cli_opt_struct! {
    RunMaintainerOpts {
        /// Address of the Solido program.
        #[clap(long)]
        solido_program_id: Pubkey,

        /// Account that stores the data for this Solido instance.
        #[clap(long)]
        solido_address: Pubkey,

        /// Listen address and port for the http server that serves a /metrics endpoint. Defaults to 0.0.0.0:8923.
        listen: String => "0.0.0.0:8923".to_owned(),

        // The expected wait time is half the max poll interval. A max poll interval
        // of a few minutes should be plenty fast for a production deployment, but
        // for testing you can reduce this value to make the daemon more responsive,
        // to eliminate some waiting time.
        /// Maximum time to wait in seconds after there was no maintenance to perform, before checking again. Defaults to 120s
        #[clap(long)]
        max_poll_interval_seconds: u64 => 120,
    }
}

impl From<&ProposeChangeMultisigOpts> for CreateMultisigOpts {
    fn from(opts: &ProposeChangeMultisigOpts) -> CreateMultisigOpts {
        CreateMultisigOpts {
            threshold: opts.threshold,
            owners: opts.owners.clone(),
            multisig_program_id: opts.multisig_program_id,
        }
    }
}
