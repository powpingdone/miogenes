import 'package:flutter/material.dart';
import 'package:frontend/ffi.dart';
import 'package:frontend/main.dart';
import 'package:provider/provider.dart';
import 'package:uuid/uuid.dart';

class MainPage extends StatefulWidget {
  const MainPage({super.key});

  @override
  State<MainPage> createState() => _MainPageState();
}

class _MainPageState extends State<MainPage> with TickerProviderStateMixin {
  late AnimationController _spinner;
  Future<Albums>? albums;

  @override
  void initState() {
    super.initState();
    _spinner = AnimationController(
        vsync: this, duration: const Duration(seconds: 2, milliseconds: 500))
      ..addListener(() {
        setState(() {});
      })
      ..repeat();
  }

  @override
  void dispose() {
    super.dispose();
    _spinner.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final mtl = Provider.of<MioTopLevel>(context);
    var mioState = mtl.mioClient;
    albums ??= mioState.getAlbums();

    return FutureBuilder(
        future: albums,
        builder: (context, snapshot) {
          if (snapshot.hasError) {
            return Text(
                "Could not fetch albums: ${extractMsg(snapshot.error)}");
          } else if (snapshot.connectionState == ConnectionState.done &&
              snapshot.hasData) {
            List<UuidValue> albums = (snapshot.data)?.albums ?? [];

            return GridView.count(
              crossAxisCount: 3,
              children: [for (UuidValue album in albums) AlbumPreview(album)],
            );
          } else {
            // show checking
            _spinner.forward();
            return CircularProgressIndicator(
              value: _spinner.value,
            );
          }
        });
  }
}

class AlbumPreview extends StatefulWidget {
  const AlbumPreview(
    this.albumId, {
    super.key,
  });

  final UuidValue albumId;

  @override
  State<AlbumPreview> createState() => _AlbumPreviewState();
}

class _AlbumPreviewState extends State<AlbumPreview>
    with TickerProviderStateMixin {
  late AnimationController _spinner;
  Future<Album>? album;
  Future<Track>? sampleTrack;

  @override
  void initState() {
    super.initState();
    _spinner = AnimationController(
        vsync: this, duration: const Duration(seconds: 2, milliseconds: 500))
      ..addListener(() {
        setState(() {});
      })
      ..repeat();
  }

  @override
  void dispose() {
    super.dispose();
    _spinner.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final mtl = Provider.of<MioTopLevel>(context);
    var mioState = mtl.mioClient;
    album ??= mioState.getAlbum(id: widget.albumId);

    return FutureBuilder(
        future: album,
        builder: (context, albumSnapshot) {
          if (albumSnapshot.hasError) {
            return Text(
                "Could not fetch album: ${extractMsg(albumSnapshot.error)}");
          } else if (albumSnapshot.connectionState == ConnectionState.done &&
              albumSnapshot.hasData) {
            UuidValue? track = (albumSnapshot.data)?.tracks[0];
            if (track != null) {
              sampleTrack ??= mioState.getTrack(id: track);
            }
            return FutureBuilder(
                future: sampleTrack,
                builder: (context, trackSnapshot) {
                  return Column(children: [
                    CoverArtImg(trackSnapshot.data),
                    Text((albumSnapshot.data as Album).title),
                    ArtistText(trackSnapshot.data)
                  ]);
                });
          } else {
            _spinner.forward();
            return CircularProgressIndicator(
              value: _spinner.value,
            );
          }
        });
  }
}

class CoverArtImg extends StatefulWidget {
  const CoverArtImg(
    this.data, {
    super.key,
  });

  final Track? data;

  @override
  State<CoverArtImg> createState() => _CoverArtImgState();
}

class _CoverArtImgState extends State<CoverArtImg> {
  Future<CoverArt>? coverArt;

  @override
  Widget build(BuildContext context) {
    final mtl = Provider.of<MioTopLevel>(context);
    var mioState = mtl.mioClient;
    if (widget.data?.coverArt != null) {
      coverArt ??= mioState.getCoverArt(id: widget.data!.coverArt!);
    }
    return FutureBuilder(
        future: coverArt,
        builder: ((context, snapshot) {
          if (snapshot.hasData) {
            return Image.memory(snapshot.data!.webmBlob);
          }
          return Container();
        }));
  }
}

class ArtistText extends StatefulWidget {
  const ArtistText(
    this.data, {
    super.key,
  });

  final Track? data;

  @override
  State<ArtistText> createState() => _ArtistTextState();
}

class _ArtistTextState extends State<ArtistText> {
  Future<Artist>? artist;

  @override
  Widget build(BuildContext context) {
    final mtl = Provider.of<MioTopLevel>(context);
    var mioState = mtl.mioClient;
    if (widget.data?.artist != null) {
      artist ??= mioState.getArtist(id: widget.data!.artist!);
    }
    return FutureBuilder(
        future: artist,
        builder: (context, snapshot) {
          if (snapshot.hasData) {
            return Text(snapshot.data!.name);
          }
          return const Text("...");
        });
  }
}
