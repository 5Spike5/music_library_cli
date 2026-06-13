use crate::models::Song;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Library {
    pub songs: Vec<Song>,
}

impl Library {
    pub fn is_empty(&self) -> bool {
        self.songs.is_empty()
    }

    pub fn len(&self) -> usize {
        self.songs.len()
    }

    pub fn get(&self, index: usize) -> Option<&Song> {
        self.songs.get(index)
    }

    pub fn next_index(&self, current: usize) -> Option<usize> {
        if self.is_empty() {
            return None;
        }

        Some((current + 1) % self.len())
    }

    pub fn previous_index(&self, current: usize) -> Option<usize> {
        if self.is_empty() {
            return None;
        }

        if current == 0 {
            Some(self.len() - 1)
        } else {
            Some(current - 1)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn library_with_three_songs() -> Library {
        Library {
            songs: vec![
                Song {
                    id: 1,
                    title: "A".to_string(),
                    path: PathBuf::from("a.mp3"),
                },
                Song {
                    id: 2,
                    title: "B".to_string(),
                    path: PathBuf::from("b.mp3"),
                },
                Song {
                    id: 3,
                    title: "C".to_string(),
                    path: PathBuf::from("c.mp3"),
                },
            ],
        }
    }

    #[test]
    fn next_index_wraps_to_first_song() {
        let library = library_with_three_songs();

        assert_eq!(library.next_index(0), Some(1));
        assert_eq!(library.next_index(2), Some(0));
    }

    #[test]
    fn previous_index_wraps_to_last_song() {
        let library = library_with_three_songs();

        assert_eq!(library.previous_index(2), Some(1));
        assert_eq!(library.previous_index(0), Some(2));
    }
}
