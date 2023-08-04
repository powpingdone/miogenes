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
                    return Padding(
                      padding: const EdgeInsets.all(8.0),
                      child: Row(mainAxisSize: MainAxisSize.min, children: [
                        Container(
                            alignment: Alignment.topLeft,
                            child: CoverArtImg(track.coverArt, size: 84)),
                        const Padding(padding: EdgeInsets.all(8.0)),
                        TitleArtistAlbumText(
                            artist: track.artist,
                            album: track.album,
                            title: track.title,
                            minified: widget.minified),
                      ]),
                    );
                  } else {
                    return Padding(
                      padding: const EdgeInsets.all(8.0),
                      child: Column(
                        mainAxisAlignment: MainAxisAlignment.center,
                        mainAxisSize: MainAxisSize.min,
                        crossAxisAlignment: CrossAxisAlignment.center,
                        children: [
                          CoverArtImg(track.coverArt, size: 300), // Cover Art
                          Padding(
                            padding:
                                const EdgeInsets.only(top: 24.0, bottom: 8.0),
                            child: TitleArtistAlbumText(
                              title: track.title,
                              album: track.album,
                              artist: track.artist,
                              minified: widget.minified,
                            ),
                          ),
                          MediaControls(
                            paused: playerStatus.data!.paused,
                          ), // Play/Pause, Next
                          VolumeSlider(
                            vol: playerStatus.data!.volume,
                          ), // Volume Control
                        ],
                      ),
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
    required this.minified,
  });

  final String title;
  final UuidValue? artist, album;
  final bool minified;

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

    final title = Text(
      widget.title,
      style: const TextStyle(fontSize: 20, fontWeight: FontWeight.bold),
      overflow: TextOverflow.ellipsis,
    );
    final artist = FutureBuilder(
        future: artistFetch,
        builder: (context, snapshot) {
          if (snapshot.hasData) {
            return Text(
              snapshot.data!.name,
              overflow: TextOverflow.ellipsis,
            );
          } else if (snapshot.hasError) {
            return const Text("?");
          } else {
            return const Text("...");
          }
        });
    final album = FutureBuilder(
        future: albumFetch,
        builder: (context, snapshot) {
          if (snapshot.hasData) {
            return Text(
              snapshot.data!.title,
              overflow: TextOverflow.ellipsis,
            );
          } else if (snapshot.hasError) {
            return const Text("?");
          } else {
            return const Text("...");
          }
        });

    // TODO: make title bigger
    if (widget.minified) {
      return LimitedBox(
        maxHeight: 84,
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          mainAxisSize: MainAxisSize.min,
          children: [
            title,
            const Padding(padding: EdgeInsets.symmetric(vertical: 4.0)),
            artist,
            album,
            Expanded(child: Container()),
          ],
        ),
      );
    } else {
      return Column(
        children: [
          title,
          const Padding(
            padding: EdgeInsets.symmetric(vertical: 4.0),
          ),
          Row(
              mainAxisAlignment: MainAxisAlignment.center,
              mainAxisSize: MainAxisSize.min,
              children: [
                artist,
                const Padding(padding: EdgeInsets.all(4.0)),
                const Text("―" /* U+2015 */),
                const Padding(padding: EdgeInsets.all(4.0)),
                album
              ])
        ],
      );
    }
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
      mainAxisAlignment: MainAxisAlignment.center,
      children: [
        IconButton(
            onPressed: () => player.toggle(),
            icon: Icon(
                paused ? Icons.play_circle_outline : Icons.pause_circle_outline,
                size: 72)),
        IconButton(
            onPressed: () => player.forward(),
            icon: const Icon(Icons.skip_next, size: 48)),
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
      mainAxisAlignment: MainAxisAlignment.center,
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
