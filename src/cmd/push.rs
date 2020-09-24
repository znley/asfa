use anyhow::{bail, Context, Result};
use clap::Clap;
// use log::info;
use std::path::{Path, PathBuf};
use std::string::String;

use crate::cfg::Config;
use crate::cmd::Command;
use crate::ssh::SshSession;
use crate::util::get_hash;

/// Upload new files.
#[derive(Clap, Debug)]
pub struct Push {
    /// Alias/file name on the remote site.
    ///
    /// If you specify multiple files to upload you can either specify no aliases or as many
    /// aliases as there are files to upload.
    #[clap(short, long)]
    alias: Vec<String>,

    /// File(s) to upload.
    #[clap()]
    files: Vec<PathBuf>,
}

fn upload(
    session: &SshSession,
    config: &Config,
    to_upload: &Path,
    target_name: &str,
) -> Result<()> {
    let mut target = session.host.folder.clone();
    let prefix_length = 
        session.host.prefix_length.unwrap_or(config.prefix_length);
    let hash = get_hash(
        to_upload,
        prefix_length
    )
    .with_context(|| format!("Could not read {} to compute hash.", to_upload.display()))?;

    target.push(&hash);
    let folder = target.clone();
    session.make_folder(&folder)?;

    target.push(target_name);

    // TODO: Maybe check if file exists already.
    session.upload_file(&to_upload, &target)?;

    if config.verify_via_hash {
        let remote_hash = session.get_remote_hash(&target, prefix_length)?;
        if hash != remote_hash {
            session.remove_folder(&folder)?;
            bail!(
                "[{}] Hashes differ: local={} remote={}",
                to_upload.display(),
                hash,
                remote_hash
            );
        }
    }

    if let Some(group) = &session.host.group {
        session.adjust_group(&folder, &group)?;
    };

    println!("{}/{}/{}", session.host.url, &hash, target_name);
    Ok(())
}

impl Command for Push {
    fn run(&self, session: &SshSession, config: &Config) -> Result<()> {
        let mut aliases: Vec<String> = vec![];

        if self.files.len() == 0 && self.alias.len() == 0 {
            bail!("No files to upload specified.");
        } else if self.files.len() == 0 && self.alias.len() > 0 {
            bail!(
                "No files to upload specified. \
                  Did you forget to seperate --alias option via double dashes from files to upload?"
            );
        } else if self.alias.len() > 0 && self.alias.len() != self.files.len() {
            bail!("You need to specify as many aliases as you specify files!");
        } else if self.alias.len() == 0 {
            for file in self.files.iter() {
                aliases.push(
                    file.file_name()
                        .with_context(|| format!("{} has no filename.", file.display()))?
                        .to_str()
                        .with_context(|| format!("{} has invalid filename", file.display()))?
                        .to_string(),
                );
            }
        } else {
            aliases = self.alias.clone();
        }

        for (to_upload, alias) in self.files.iter().zip(aliases.iter()) {
            upload(session, config, to_upload, alias)?;
        }

        Ok(())
    }
}
