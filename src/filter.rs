use std::path::{Path, PathBuf};

use globset::{Glob, GlobSet, GlobSetBuilder};

use crate::defaults::DEFAULT_EXCLUDES;
use crate::error::Error;

pub struct FileFilter {
    include_set: Option<GlobSet>,
    exclude_set: GlobSet,
}

impl FileFilter {
    pub fn new(
        include_patterns: &[String],
        exclude_patterns: &[String],
    ) -> Result<Self, Error> {
        let include_set = if include_patterns.is_empty() {
            None
        } else {
            let set = include_patterns
                .iter()
                .try_fold(GlobSetBuilder::new(), |mut b, p| {
                    b.add(Glob::new(p).map_err(|e| Error::Filter(e.to_string()))?);
                    Ok::<_, Error>(b)
                })?
                .build()
                .map_err(|e| Error::Filter(e.to_string()))?;
            Some(set)
        };

        let exclude_set = DEFAULT_EXCLUDES
            .iter()
            .map(|p| Glob::new(p).unwrap())
            .chain(
                exclude_patterns
                    .iter()
                    .map(|p| Glob::new(p))
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|e| Error::Filter(e.to_string()))?
                    .into_iter(),
            )
            .fold(GlobSetBuilder::new(), |mut b, g| {
                b.add(g);
                b
            })
            .build()
            .map_err(|e| Error::Filter(e.to_string()))?;

        Ok(Self {
            include_set,
            exclude_set,
        })
    }

    pub fn should_include(&self, path: &Path) -> bool {
        if self.exclude_set.is_match(path) {
            return false;
        }
        self.include_set
            .as_ref()
            .map_or(true, |set| set.is_match(path))
    }

    pub fn filter_paths(&self, paths: Vec<PathBuf>) -> impl Iterator<Item = PathBuf> + '_ {
        paths.into_iter().filter(|p| self.should_include(p))
    }
}

pub fn is_binary(content: &[u8]) -> bool {
    content_inspector::inspect(content).is_binary()
}

pub fn is_minified(content: &str) -> bool {
    content.lines().take(5).any(|line| line.len() > 500)
}
