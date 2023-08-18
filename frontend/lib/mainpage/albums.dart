import 'dart:math';

import 'package:flutter/material.dart';
import 'package:flutter_spinkit/flutter_spinkit.dart';
import 'package:frontend/ffi.dart';
import 'package:frontend/main.dart';
import 'package:frontend/mainpage/toplevel.dart';
import 'package:provider/provider.dart';
import 'package:uuid/uuid.dart';

class AlbumPage extends StatelessWidget {
  const AlbumPage({
    super.key,
  });

  @override
  Widget build(BuildContext context) {
    var mainState = Provider.of<MainNavTopLevel>(context);
    return FutureBuilder(
        future: mainState.albums,
        builder: (context, snapshot) {
          if (snapshot.hasError) {
            return Text(
                "Could not fetch albums: ${extractMsg(snapshot.error)}");
          } else if (snapshot.connectionState == ConnectionState.done &&
              snapshot.hasData) {
            List<UuidValue> albums = (snapshot.data)?.albums ?? [];

            return SafeArea(
              child: GridView.extent(
                maxCrossAxisExtent: 300,
                children: [for (UuidValue album in albums) AlbumPreview(album)],
              ),
            );
          } else {
            // show checking
            return SpinKitWanderingCubes(
              color: Theme.of(context).colorScheme.primary,
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

class _AlbumPreviewState extends State<AlbumPreview> {
  Future<Album>? album;
  Future<Track>? sampleTrack;

  @override
  Widget build(BuildContext context) {
    final player = Provider.of<MioPlayerState>(context).mioPlayer;
    final mioState = Provider.of<MioTopLevel>(context).mioClient;
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
                  return Padding(
                    padding: const EdgeInsets.all(4.0),
                    child: TextButton(
                        onPressed: () {
                          player.queue(
                              id: (albumSnapshot.data)!.tracks[Random().nextInt(
                                  (albumSnapshot.data!.tracks.length))]);
                          player.play();
                        },
                        style: ButtonStyle(
                            shape: MaterialStateProperty.all(
                                RoundedRectangleBorder(
                          borderRadius: BorderRadius.circular(16.0),
                        ))),
                        child: Container(
                          padding: const EdgeInsets.all(8.0),
                          child: Column(
                              crossAxisAlignment: CrossAxisAlignment.center,
                              mainAxisAlignment: MainAxisAlignment.center,
                              children: [
                                Flexible(flex: 0, child: Container()),
                                Flexible(
                                    child: CoverArtImg(
                                        trackSnapshot.data?.coverArt,
                                        size: 200)),
                                Container(
                                  width: 200,
                                  alignment: Alignment.topCenter,
                                  child: Text(
                                      (albumSnapshot.data as Album).title,
                                      overflow: TextOverflow.ellipsis,
                                      style: const TextStyle(
                                          fontWeight: FontWeight.bold)),
                                ),
                                Container(
                                    width: 200,
                                    alignment: Alignment.topCenter,
                                    child: ArtistText(trackSnapshot.data)),
                                Flexible(flex: 0, child: Container()),
                              ]),
                        )),
                  );
                });
          } else {
            return SpinKitWanderingCubes(
              color: Theme.of(context).colorScheme.primary,
            );
          }
        });
  }
}

class CoverArtImg extends StatefulWidget {
  const CoverArtImg(
    this.coverArtId, {
    super.key,
    this.size,
  });

  final UuidValue? coverArtId;
  final double? size;

  @override
  State<CoverArtImg> createState() => _CoverArtImgState();
}

class _CoverArtImgState extends State<CoverArtImg> {
  Future<CoverArt>? coverArt;

  @override
  Widget build(BuildContext context) {
    final mtl = Provider.of<MioTopLevel>(context);
    var mioState = mtl.mioClient;
    if (widget.coverArtId != null) {
      coverArt ??= mioState.getCoverArt(id: widget.coverArtId!);
      return FutureBuilder(
          future: coverArt,
          builder: ((context, snapshot) {
            if (snapshot.hasData) {
              return FittedBox(
                clipBehavior: Clip.hardEdge,
                child: Image.memory(
                  snapshot.data!.webmBlob,
                  fit: BoxFit.cover,
                  isAntiAlias: true,
                  width: widget.size,
                  height: widget.size,
                ),
              );
            }
            // TODO: show error and loading image
            return Container();
          }));
    } else {
      return FittedBox(
        clipBehavior: Clip.antiAlias,
        child: Container(
            width: widget.size,
            height: widget.size,
            color: Theme.of(context).disabledColor,
            child: Icon(
              Icons.image_not_supported,
              size: widget.size == null ? 96 : widget.size! / 2,
            )),
      );
    }
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
            return Text(
              snapshot.data!.name,
              overflow: TextOverflow.ellipsis,
              style: const TextStyle(fontSize: 12),
            );
          }
          return const Text("...");
        });
  }
}
