use crate::build::container::{checked_exec, Context};
use crate::container::ExecOpts;
use crate::{Error, Result};

use std::path::PathBuf;
use tracing::{debug, info, info_span, trace, Instrument};

macro_rules! run_script {
    ($phase:literal, $script:expr, $dir:expr,  $ctx:ident) => {{
        let _span = info_span!($phase);
        async move {
            trace!(script = ?$script);
            info!(concat!("executing ", $phase, " scripts"));
            let mut opts = ExecOpts::default();
            let mut _dir;

            if let Some(dir) = &$script.working_dir {
                trace!(working_dir = %dir.display());
                let dir_s = dir.to_string_lossy();
                let bld_dir = $ctx.build_ctx.container_bld_dir.to_string_lossy();
                let out_dir = $ctx.build_ctx.container_out_dir.to_string_lossy();
                let mut dir_s = dir_s.replace("$PKGER_BLD_DIR", &bld_dir);
                dir_s = dir_s.replace("$PKGER_OUT_DIR", &out_dir);
                _dir = PathBuf::from(dir_s);
                opts = opts.working_dir(_dir.as_path());
            } else {
                trace!(working_dir = %$dir.display(), "using default");
                opts = opts.working_dir($dir);
            }

            if let Some(shell) = &$script.shell {
                trace!(shell = %shell);
                opts = opts.shell(shell.as_str());
            }

            for cmd in &$script.steps {
                if let Some(images) = &cmd.images {
                    trace!(images = ?images, "only execute on");
                    if !images.contains(&$ctx.build_ctx.target.image().to_owned()) {
                        trace!(image = %$ctx.build_ctx.target.image(), "not found in images");
                        if !cmd.has_target_specified() {
                            debug!(command = %cmd.cmd, "skipping, excluded by image filter");
                            continue;
                        }
                    }
                }

                if !cmd.should_run_on($ctx.build_ctx.target.build_target()) {
                    debug!(command = %cmd.cmd, "skipping, shouldn't run on target");
                    continue;
                }

                debug!(command = %cmd.cmd, "running");
                checked_exec(&$ctx, &opts.clone().cmd(&cmd.cmd).build())
                    .await?;
            }

            Ok::<_, Error>(())
        }
        .instrument(_span)
        .await?;
    }};
}

pub async fn execute_scripts(ctx: &Context<'_>) -> Result<()> {
    let span = info_span!("exec-scripts");
    async move {
        if let Some(config_script) = &ctx.build_ctx.recipe.configure_script {
            run_script!(
                "configure",
                config_script,
                &ctx.build_ctx.container_bld_dir,
                ctx
            );
        } else {
            info!("no configure steps to run");
        }

        let build_script = &ctx.build_ctx.recipe.build_script;
        run_script!("build", build_script, &ctx.build_ctx.container_bld_dir, ctx);

        if let Some(install_script) = &ctx.build_ctx.recipe.install_script {
            run_script!(
                "install",
                install_script,
                &ctx.build_ctx.container_out_dir,
                ctx
            );
        } else {
            info!("no install steps to run");
        }

        Ok(())
    }
    .instrument(span)
    .await
}
