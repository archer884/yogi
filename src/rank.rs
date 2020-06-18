use hashbrown::HashSet;
use std::path::Path;

#[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq)]
pub struct Rank {
    segments: usize,
    words: usize,
}

pub struct PathRanker {
    dictionary: HashSet<&'static str>,
}

impl PathRanker {
    pub fn new() -> Self {
        let words = include_str!("../resource/enable1.txt");
        Self {
            dictionary: words.split_whitespace().collect(),
        }
    }

    pub fn rank(&self, path: impl AsRef<Path>) -> Rank {
        let utf8_segments = path
            .as_ref()
            .components()
            .filter_map(|x| x.as_os_str().to_str());
        let words = self.count_words(utf8_segments);

        Rank {
            segments: path.as_ref().components().count(),
            words,
        }
    }

    fn count_words<'a>(&self, segments: impl IntoIterator<Item = &'a str>) -> usize {
        let candidates = segments.into_iter().flat_map(|x| x.split_whitespace());
        candidates
            .map(|s| s.to_lowercase())
            .filter(|x| self.dictionary.contains(x.as_str()))
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::PathRanker;

    #[test]
    fn filenames_with_words_outrank_filenames_without() {
        let word_free = "./59670723_2289729794623117_1948407069107290112_n.mp4";
        let with_words = "./Video by foo-Bw8_9u_lqjt.mp4";
        let ranker = PathRanker::new();
        let a = ranker.rank(with_words);
        let b = ranker.rank(word_free);
        assert!(a > b);
    }

    #[test]
    fn segments_outweigh_words() {
        let ranker = PathRanker::new();
        let a = ranker.rank("123/1234.png");
        let b = ranker.rank("Hello world.png");
        assert!(a > b);
    }
}
