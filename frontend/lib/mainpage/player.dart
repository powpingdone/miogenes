import "package:flutter/material.dart";
import "package:flutter_spinkit/flutter_spinkit.dart";
import "package:frontend/ffi.dart";
import "package:frontend/main.dart";
import "package:frontend/mainpage/albums.dart";
import "package:frontend/mainpage/toplevel.dart";
import "package:provider/provider.dart";
import "package:uuid/uuid.dart";

class Player extends StatefulWidget {
  const Player({super.key});

  @override
  State<Player> createState() => _PlayerState();
}

class _PlayerState extends State<Player> {
  Stream<PStatus>? stream;
  UuidValue currFetched = UuidValue("00000000-0000-0000-0000-000000000000");
  Future<Track>? fetchTrack;

  @override
  Widget build(BuildContext context) {
    var mioState = Provider.of<MioTopLevel>(context).mioClient;
    var player = Provider.of<MioPlayerState>(context).mioPlayer;
    stream ??= player.infoStream();
    return StreamBuilder(
        stream: stream,
        builder: (context, snapshot) {
          if (snapshot.hasError) {
            throw UnimplementedError();
          } else if (!snapshot.hasData) {
            return SpinKitWanderingCubes(
              color: Theme.of(context).colorScheme.primary,
            );
          }
          // this is allowed to be late data, too.
          PStatus data = snapshot.data!;

          // if data has error
          if (data.errMsg != null) {
            return Column(
              children: [
                const Text("An error has occurred."),
                Text("${data.errMsg}"),
                const Text(
                    "This may be a bug in the software itself. You may wish to report it at https://github.com/powpingdone/miogenes"),
              ],
            );
          }

          // is there a "currently playing" track?
          if (data.queue.isEmpty) {
            // TODO: return equiv layout to a "currently playing" layout
            return const Text("Not currently playing...");
          }

          // begin fetch
          if (data.queue.first != currFetched) {
            currFetched = data.queue.first;
            fetchTrack = mioState.getTrack(id: currFetched);
          }

          return FutureBuilder(
              future: fetchTrack,
              builder: (context, fetchShot) {
                if (fetchShot.hasError) {
                  return Text(
                      "Error encountered: ${extractMsg(fetchShot.error)}.");
                } else if (fetchShot.connectionState == ConnectionState.done &&
                    fetchShot.hasData) {
                  Track track = fetchShot.data!;
                  return Row(
                    children: [
                      CoverArtImg(track.coverArt), // Cover Art
                      TitleArtistAlbumText(
                        title: track.title,
                        album: track.album,
                        artist: track.artist,
                      ),
                      Container(), // Play/Pause, Next
                      Container(), // Volume Control
                    ],
                  );
                } else {
                  // TODO: return equiv layout to a "currently playing" layout
                  return const Text("Loading...");
                }
              });
        });
  }
}

class TitleArtistAlbumText extends StatefulWidget {
  const TitleArtistAlbumText({
    super.key,
    required this.artist,
    required this.album,
    required this.title,
  });

  final String title;
  final UuidValue? artist, album;

  @override
  State<TitleArtistAlbumText> createState() => _TitleArtistAlbumTextState();
}

class _TitleArtistAlbumTextState extends State<TitleArtistAlbumText> {
  Future<Album>? albumFetch;
  Future<Artist>? artistFetch;

  @override
  Widget build(BuildContext context) {
    var mioState = Provider.of<MioTopLevel>(context).mioClient;
    albumFetch ??=
        widget.album != null ? mioState.getAlbum(id: widget.album!) : null;
    artistFetch ??=
        widget.artist != null ? mioState.getArtist(id: widget.artist!) : null;

    return Row(
      children: [
        Text(widget.title),
        Column(children: [
          FutureBuilder(future: artistFetch,builder: (context, snapshot) {
            if (snapshot.hasData) {
              return Text("${snapshot.data}");
            } else if (snapshot.hasError) {
              return const Text("?");
            } else {
              return const Text("...");
            }
          }),
          const Text("―"),
          FutureBuilder(future:albumFetch, builder: (context, snapshot) {
            if (snapshot.hasData) {
              return Text("${snapshot.data}");
            } else if (snapshot.hasError) {
              return const Text("?");
            } else {
              return const Text("...");
            }
          })
        ])
      ],
    );
  }
}
