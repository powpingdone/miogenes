use mio_common::retstructs;

// send out the type
pub trait WebOut {
    type WebOut;
    fn web_out(self) -> Self::WebOut;
}

impl WebOut for mio_entity::Track {
    type WebOut = retstructs::Track;
    fn web_out(self) -> Self::WebOut {
        retstructs::Track {
            title: self.title,
            album: self.album,
            cover_art: self.cover_art,
            artist: self.artist,
            tags: self.tags,
            sort_name: self.sort_name,
        }
    }
}

impl WebOut for mio_entity::Album {
    type WebOut = retstructs::Album;
    fn web_out(self) -> Self::WebOut {
        retstructs::Album {
            artist: self.artist,
            title: self.title,
            tracks: self.tracks,
            sort_name: self.sort_name,
        }
    }
}

impl WebOut for mio_entity::CoverArt {
    type WebOut = retstructs::CoverArt;
    fn web_out(self) -> Self::WebOut {
        retstructs::CoverArt { data: self.data }
    }
}

impl WebOut for mio_entity::Artist {
    type WebOut = retstructs::Artist;

    fn web_out(self) -> Self::WebOut {
        retstructs::Artist {
            name: self.name,
            sort_name: self.sort_name,
        }
    }
}
