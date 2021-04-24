mod git;
mod image;
mod target;

pub use git::GitSource;
pub use image::ImageTarget;
pub use target::BuildTarget;

use crate::deps::Dependencies;
use crate::{Error, Result};

use serde::{Deserialize, Serialize};
use std::convert::TryFrom;

#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct MetadataRep {
    // Required
    pub name: String,
    pub version: String,
    pub description: String,
    pub license: String,
    pub images: Vec<toml::Value>,

    // Common optional
    pub maintainer: Option<String>,
    pub arch: Option<String>,
    /// http/https or file system source pointing to a tar.gz or tar.xz package
    pub source: Option<String>,
    /// Git repository as source
    pub git: Option<toml::Value>,
    /// Whether to install default dependencies before build
    pub skip_default_deps: Option<bool>,
    /// Directories to exclude when creating the package
    pub exclude: Option<Vec<String>>,
    pub group: Option<String>,

    pub build_depends: Option<toml::Value>,
    pub depends: Option<toml::Value>,
    pub conflicts: Option<toml::Value>,
    pub provides: Option<toml::Value>,

    // Only DEB
    pub deb: Option<DebRep>,

    // Only RPM
    pub rpm: Option<RpmRep>,
}

#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct DebRep {
    pub priority: Option<String>,
}

#[derive(Clone, Debug)]
pub struct DebInfo {
    pub priority: Option<String>,
}

impl From<DebRep> for DebInfo {
    fn from(rep: DebRep) -> Self {
        Self {
            priority: rep.priority,
        }
    }
}

#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct RpmRep {
    pub obsoletes: Option<toml::Value>,
    pub release: Option<String>,
    pub epoch: Option<String>,
    pub vendor: Option<String>,
    pub icon: Option<String>,
    pub summary: Option<String>,
    pub pre_script: Option<String>,
    pub post_script: Option<String>,
    pub preun_script: Option<String>,
    pub postun_script: Option<String>,
    pub config_noreplace: Option<String>,
}

impl TryFrom<RpmRep> for RpmInfo {
    type Error = Error;

    fn try_from(rep: RpmRep) -> Result<Self> {
        let obsoletes = if let Some(deps) = rep.obsoletes {
            Some(Dependencies::try_from(deps)?)
        } else {
            None
        };
        Ok(Self {
            obsoletes,
            release: rep.release,
            epoch: rep.epoch,
            vendor: rep.vendor,
            icon: rep.icon,
            summary: rep.summary,
            pre_script: rep.pre_script,
            post_script: rep.post_script,
            preun_script: rep.preun_script,
            postun_script: rep.postun_script,
            config_noreplace: rep.config_noreplace,
        })
    }
}

#[derive(Clone, Debug)]
pub struct RpmInfo {
    pub obsoletes: Option<Dependencies>,
    pub release: Option<String>,
    pub epoch: Option<String>,
    pub vendor: Option<String>,
    pub icon: Option<String>,
    pub summary: Option<String>,
    pub pre_script: Option<String>,
    pub post_script: Option<String>,
    pub preun_script: Option<String>,
    pub postun_script: Option<String>,
    pub config_noreplace: Option<String>,
}

#[derive(Clone, Debug)]
pub struct Metadata {
    // General
    pub name: String,
    pub version: String,
    pub description: String,
    pub license: String,
    pub images: Vec<ImageTarget>,

    pub maintainer: Option<String>,
    pub arch: Option<String>,
    /// http/https or file system source pointing to a tar.gz or tar.xz package
    pub source: Option<String>,
    /// Git repository as source
    pub git: Option<GitSource>,
    /// Whether default dependencies should be installed before the build
    pub skip_default_deps: Option<bool>,
    /// Directories to exclude when creating the package
    pub exclude: Option<Vec<String>>,
    /// Works as section in DEB and group in RPM
    pub group: Option<String>,

    pub build_depends: Option<Dependencies>,

    pub depends: Option<Dependencies>,
    pub conflicts: Option<Dependencies>,
    pub provides: Option<Dependencies>,

    pub deb: Option<DebInfo>,

    pub rpm: Option<RpmInfo>,
}

impl Metadata {
    /// Returns the name of `arch` appropriate for DEB build
    pub fn deb_arch(&self) -> &str {
        if let Some(arch) = &self.arch {
            match &arch[..] {
                "amd64" | "x86_64" => "amd64",
                "x86" | "i386" => "i386",
                arch => arch,
                // #TODO: add more...
            }
        } else {
            "all"
        }
    }

    /// Returns the name of `arch` appropriate for RPM build
    pub fn rpm_arch(&self) -> &str {
        if let Some(arch) = &self.arch {
            match &arch[..] {
                "amd64" | "x86_64" => "x86_64",
                "x86" | "i386" => "x86",
                arch => arch,
                // #TODO: add more...
            }
        } else {
            "noarch"
        }
    }

    /// Returns the RPM release if the value is available, otherwise returns "0"
    pub fn rpm_release(&self) -> &str {
        if let Some(rpm) = &self.rpm {
            if let Some(release) = &rpm.release {
                return release.as_str();
            }
        }

        "0"
    }
}

impl TryFrom<MetadataRep> for Metadata {
    type Error = Error;

    fn try_from(rep: MetadataRep) -> Result<Self> {
        let build_depends = if let Some(deps) = rep.build_depends {
            Some(Dependencies::try_from(deps)?)
        } else {
            None
        };
        let depends = if let Some(deps) = rep.depends {
            Some(Dependencies::try_from(deps)?)
        } else {
            None
        };
        let conflicts = if let Some(deps) = rep.conflicts {
            Some(Dependencies::try_from(deps)?)
        } else {
            None
        };
        let provides = if let Some(deps) = rep.provides {
            Some(Dependencies::try_from(deps)?)
        } else {
            None
        };

        let mut images = vec![];
        for image in rep.images.into_iter().map(ImageTarget::try_from) {
            images.push(image?);
        }

        Ok(Self {
            name: rep.name,
            version: rep.version,
            description: rep.description,
            license: rep.license,
            images,

            maintainer: rep.maintainer,
            arch: rep.arch,
            source: rep.source,
            git: {
                if let Some(val) = rep.git {
                    GitSource::try_from(val).map(Some)?
                } else {
                    None
                }
            },
            skip_default_deps: rep.skip_default_deps,
            exclude: rep.exclude,
            group: rep.group,

            build_depends,

            depends,
            conflicts,
            provides,

            deb: rep.deb.map(DebInfo::from),

            rpm: if let Some(rpm) = rep.rpm {
                Some(RpmInfo::try_from(rpm)?)
            } else {
                None
            },
        })
    }
}
