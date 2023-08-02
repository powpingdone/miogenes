import "package:flutter/material.dart";
import "package:flutter_spinkit/flutter_spinkit.dart";
import "package:frontend/ffi.dart";
import "package:frontend/main.dart";
import "package:frontend/mainpage/albums.dart";
import "package:frontend/mainpage/toplevel.dart";
import "package:provider/provider.dart";
import "package:uuid/uuid.dart";

class Player extends StatefulWidget {
  const Player({super.key, required this.minified});

  final bool minified;

  @override
  State<Player> createState() => _PlayerState();
}

class _PlayerState extends State<Player> {
  Stream<PStatus>? stream;
  UuidValue currFetched = UuidValue("00000000-0000-0000-0000-000000000000");
  Future<Track>? fetchTrack;
  String? errMsg;

  @override
  Widget build(BuildContext context) {
    final mioState = Provider.of<MioTopLevel>(context).mioClient;
    final player = Provider.of<MioPlayerState>(context).mioPlayer;
    stream ??= player.infoStream();
    return StreamBuilder(
        stream: stream,
        builder: (context, playerStatus) {
          // if data has error
          if (playerStatus.data?.errMsg != null) {
            errMsg = playerStatus.data?.errMsg;
          }
          if (errMsg != null) {
            return Column(
              children: [
                const Text("An error has occurred."),
                Text(errMsg!),
                const Text("This may be a bug in the software itself. "
                    "You may wish to report it at https://github.com/powpingdone/miogenes"),
              ],
            );
          }

          if (playerStatus.hasError && errMsg == null) {
            throw UnimplementedError(extractMsg(playerStatus.error));
          } else if (!playerStatus.hasData) {
            return SpinKitWanderingCubes(
              color: Theme.of(context).colorScheme.primary,
            );
          }
          // this is allowed to be late data, too.
          PStatus data = playerStatus.data!;

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
                  if (widget.minified) {
                    return Row(children: [
                      CoverArtImg(track.coverArt),
                      TitleArtistAlbumText(
                          artist: track.artist,
                          album: track.album,
                          title: track.title),
                    ]);
                  } else {
                    return Column(
                      children: [
                        CoverArtImg(track.coverArt), // Cover Art
                        TitleArtistAlbumText(
                          title: track.title,
                          album: track.album,
                          artist: track.artist,
                        ),
                        MediaControls(
                          paused: playerStatus.data!.paused,
                        ), // Play/Pause, Next
                        VolumeSlider(
                          vol: playerStatus.data!.volume,
                        ), // Volume Control
                      ],
                    );
                  }
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

    return Column(
      children: [
        // TODO: make title bigger
        Text(widget.title),
        Row(children: [
          FutureBuilder(
              future: artistFetch,
              builder: (context, snapshot) {
                if (snapshot.hasData) {
                  return Text(snapshot.data!.name);
                } else if (snapshot.hasError) {
                  return const Text("?");
                } else {
                  return const Text("...");
                }
              }),
          const Text("―" /* U+2015 */),
          FutureBuilder(
              future: albumFetch,
              builder: (context, snapshot) {
                if (snapshot.hasData) {
                  return Text(snapshot.data!.title);
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

class MediaControls extends StatelessWidget {
  const MediaControls({
    super.key,
    required this.paused,
  });

  final bool paused;

  @override
  Widget build(BuildContext context) {
    var player = Provider.of<MioPlayerState>(context).mioPlayer;
    return Row(
      children: [
        IconButton(
            onPressed: () => player.toggle(),
            icon: Icon(paused
                ? Icons.pause_circle_outline
                : Icons.play_circle_outline)),
        IconButton(
            onPressed: () => player.forward(),
            icon: const Icon(Icons.skip_next)),
      ],
    );
  }
}

class VolumeSlider extends StatelessWidget {
  const VolumeSlider({
    super.key,
    required this.vol,
  });

  final double vol;

  @override
  Widget build(BuildContext context) {
    var player = Provider.of<MioPlayerState>(context).mioPlayer;
    return Row(
      children: [
        const Icon(Icons.volume_up),
        Slider(
            value: vol,
            min: 0.0,
            max: 1.0,
            onChanged: (newVol) => player.volume(volume: newVol)),
      ],
    );
  }
}
