use std::{env, path::{Path, PathBuf}, collections::HashMap, time::SystemTime};

use anyhow::{Result, Context, anyhow};
use console::style;
use pathdiff::diff_paths;
use tokio::{fs, io::AsyncWriteExt};
use walkdir::WalkDir;

use crate::model::BootstrappedFile;

use super::BuildContext;

impl BuildContext {
    pub async fn bootstrap_files(&mut self) -> Result<()> {
        if !Path::new(self.output_dir.as_path()).exists() {
            fs::create_dir_all(&self.output_dir)
                .await
                .context("Creating output directory (server/)")?;
        }

        let lockfile_entries: HashMap<PathBuf, SystemTime> = HashMap::from_iter(self.lockfile.files.iter()
            .map(|e| (e.path.clone(), e.date.clone())));

        for entry in WalkDir::new(self.server.path.join("config")) {
            let entry = entry
                .map_err(|e| anyhow!(
                    "Can't walk directory/file: {}",
                    &e.path().unwrap_or(Path::new("<unknown>")
                ).display()))?;
    
            if entry.file_type().is_dir() {
                continue;
            }

            let source = entry.path();
            let dest = self.map_config_path(source);
            let diffed_paths = diff_paths(&dest, self.server.path.join("config"))
                .ok_or(anyhow!("Cannot diff paths"))?;
    
            self.bootstrap_file(&diffed_paths, lockfile_entries.get(&diffed_paths)).await.context(format!(
                "Bootstrapping file: {}",
                entry.path().display()
            ))?;
        }

        if self.server.launcher.eula_args && !self.server.jar.supports_eula_args() {
            println!(
                "          {}",
                style("=> eula.txt [eula_args unsupported]").dim()
            );
            fs::File::create(self.output_dir.join("eula.txt"))
                .await?
                .write_all(b"eula=true\n")
                .await?;
        }

        println!("          {}", style("Bootstrapping complete").dim());

        Ok(())
    }

    pub fn map_config_path(&self, path: &Path) -> PathBuf {
        let mut path_buf = PathBuf::new();
        let mut iter = path.components();

        iter.next().expect("Path to have atleast 1 component");

        path_buf.push(self.output_dir.as_os_str());

        path_buf.extend(iter);

        path_buf
    }

    pub fn should_bootstrap_file(&self, path: &Path) -> bool {
        let ext = path.extension().unwrap_or_default().to_str().unwrap_or_default();

        let bootstrap_exts = vec![
            "properties", "txt", "yaml", "yml", "conf", "config", "toml", "json", "json5", "secret"
        ];

        bootstrap_exts.contains(&ext)
    }

    pub async fn bootstrap_file(&mut self, path: &PathBuf, cache: Option<&SystemTime>) -> Result<()> {
        let pretty_path = path.display();

        let source = self.server.path.join("config").join(path);
        let dest = self.output_dir.join(path);

        let source_time = fs::metadata(&source).await?.modified()?;

        if self.force || {
            if let Some(time) = cache {
                &source_time > time
            } else {
                true
            }
        } {
            if self.should_bootstrap_file(path) {
                let config_contents = fs::read_to_string(&source)
                    .await.context(format!("Reading from '{}' ; [{pretty_path}]", source.display()))?;
    
                let bootstrapped_contents = self.bootstrap_content(&config_contents);
    
                fs::write(&dest, bootstrapped_contents)
                    .await.context(format!("Writing to '{}' ; [{pretty_path}]", dest.display()))?;
            } else {
                // ? idk why but read_to_string and fs::write works with 'dest' but fs::copy doesnt
                fs::copy(&source, &dest)
                    .await.context(format!("Copying '{}' to '{}' ; [{pretty_path}]", source.display(), dest.display()))?;
            }
    
            println!(
                "          {}",
                style(format!("-> {pretty_path}")).dim()
            );
        } else {
            println!(
                "          {}",
                style(format!("unchanged: {pretty_path}")).dim()
            );
        }

        self.new_lockfile.files.push(BootstrappedFile {
            path: path.clone(),
            date: source_time
        });

        Ok(())
    }

    pub fn bootstrap_content(&self, content: &str) -> String {
        mcapi::dollar_repl(content, |k| {
            let k = k.trim();

            let (k, def) = if let Some((k, def)) = k.split_once(':') {
                (k.trim(), Some(def.trim().to_owned()))
            } else {
                (k, None)
            };

            match k {
                "SERVER_NAME" => Some(self.server.name.clone()),
                "SERVER_VERSION" | "mcver" | "mcversion" => Some(self.server.mc_version.clone()),
                "PLUGIN_COUNT" => Some(self.server.plugins.len().to_string()),
                "MOD_COUNT" => Some(self.server.mods.len().to_string()),
                "WORLD_COUNT" => Some(self.server.worlds.len().to_string()),
                "CLIENTSIDE_MOD_COUNT" => Some(self.server.clientsidemods.len().to_string()),
                k => self.server.variables.get(k)
                    .cloned()
                    .or(env::var(k).ok())
            }.or(def)
        })
    }
}
