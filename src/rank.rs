use std::path::Path;

use hashbrown::HashSet;
use regex::Regex;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Rank {
    segments: i32,
    words: i32,
    is_unique: bool,
}

pub struct PathRanker {
    dictionary: HashSet<&'static str>,
    pattern: Regex,
}

impl PathRanker {
    pub fn new() -> Self {
        let words = include_str!("../resource/enable1.txt");
        Self {
            dictionary: words.split_whitespace().collect(),
            pattern: Regex::new(r#"\(\d+\)"#).unwrap(),
        }
    }

    pub fn rank(&self, path: impl AsRef<Path>) -> Rank {
        let path = path.as_ref();
        let utf8_segments = path.components().filter_map(|x| x.as_os_str().to_str());

        let words = self.evaluate_segments(utf8_segments);

        Rank {
            segments: path.components().count() as i32,
            words,
            is_unique: !path
                .file_name()
                .and_then(|file_name| file_name.to_str())
                .map(|file_name| self.pattern.is_match(file_name))
                .unwrap_or_default(),
        }
    }

    fn evaluate_segments<'a>(&self, segments: impl IntoIterator<Item = &'a str>) -> i32 {
        static NON_WHITESPACE_SPLIT_CHARS: &[char] = &['.', '_', '-'];

        let candidates = segments.into_iter().flat_map(|x| {
            x.split(|u: char| u.is_whitespace() || NON_WHITESPACE_SPLIT_CHARS.contains(&u))
        });

        candidates
            .map(|s| s.to_lowercase())
            .filter(|x| self.dictionary.contains(x.as_str()))
            .count() as i32
    }
}

#[cfg(test)]
mod tests {
    use super::PathRanker;

    #[test]
    fn filenames_with_words_outrank_filenames_without() {
        let with_words = "./Video by foo-Bw8_9u_lqjt.mp4";
        let word_free = "./59670723_2289729794623117_1948407069107290112_n.mp4";
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

    #[test]
    fn files_distinguished_by_parenthetical_numbers_rank_lower() {
        let ranker = PathRanker::new();
        let original = dbg!(ranker.rank("hello.jpg"));
        let duplicate = dbg!(ranker.rank("hello (1).jpg"));
        assert!(original > duplicate);
    }
}
