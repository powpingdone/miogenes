import "dart:ffi";

import "package:flutter/material.dart";
import "package:flutter_spinkit/flutter_spinkit.dart";
import "package:frontend/ffi.dart";
import "package:frontend/main.dart";
import "package:frontend/mainpage/albums.dart";
import "package:frontend/mainpage/toplevel.dart";
import "package:provider/provider.dart";
import "package:text_scroll/text_scroll.dart";
import "package:uuid/uuid.dart";
import "package:audio_service/audio_service.dart";

class Player extends StatefulWidget {
  const Player({super.key});

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
    final player = Provider.of<MioPlayerState>(context);
    return StreamBuilder(
        stream: player.playbackState.stream,
        builder: (context, playerStatus) {
          if (playerStatus.hasError && errMsg == null) {
            throw UnimplementedError(extractMsg(playerStatus.error));
          } else if (!playerStatus.hasData) {
            return SpinKitWanderingCubes(
              color: Theme.of(context).colorScheme.primary,
            );
          }
          // pickup player state, and currently playing
          final PlaybackState data = playerStatus.data!;
          final pb = player.mediaItem;

          // is there a "currently playing" track?
          if (pb.value == null) {
            // TODO: return equiv layout to a "currently playing" layout
            return Container(
                alignment: Alignment.topLeft,
                padding: const EdgeInsets.symmetric(vertical: 8, horizontal: 8),
                child: const Center(child: Text("Not currently playing...")));
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
                          ),
                        ),
                        MediaControls(
                          paused: playerStatus.data!.paused,
                        ), // Play/Pause, Next
                        DurationSlider(
                          atDur: playerStatus.data!.volume,
                        ), // Volume Control
                      ],
                    ),
                  );
                } else {
                  return Container(
                      alignment: Alignment.topLeft,
                      padding: const EdgeInsets.symmetric(
                          vertical: 8, horizontal: 8),
                      child: const Text("Loading..."));
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
  // TODO: use currently playing mdata
  //Future<List<String?>>? albumArtistFetch;

  @override
  Widget build(BuildContext context) {
    var mioPlayer = Provider.of<MioPlayerState>(context);
    final title = TextScroll(
      widget.title,
      mode: TextScrollMode.bouncing,
      style: const TextStyle(fontSize: 20, fontWeight: FontWeight.bold),
    );
    return StreamBuilder(
        stream: mioPlayer.mediaItem.stream,
        builder: (context, snapshot) {
          final album =
              snapshot.data?.album == null ? "..." : snapshot.data!.album!;
          final artist =
              snapshot.data?.artist == null ? "..." : snapshot.data!.artist!;
          return Column(
            children: [
              title,
              const Padding(
                padding: EdgeInsets.symmetric(vertical: 4.0),
              ),
              TextScroll(
                artist,
                mode: TextScrollMode.endless,
              ),
              TextScroll(album, mode: TextScrollMode.endless),
            ],
          );
        });
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
    var player = Provider.of<MioPlayerState>(context);
    return Row(
      mainAxisAlignment: MainAxisAlignment.center,
      children: [
        IconButton(
            onPressed: () async => await player.toggle(),
            icon: Icon(
                paused ? Icons.play_circle_outline : Icons.pause_circle_outline,
                size: 72)),
        IconButton(
            onPressed: () async => await player.skipToNext(),
            icon: const Icon(Icons.skip_next, size: 48)),
      ],
    );
  }
}

class DurationSlider extends StatelessWidget {
  const DurationSlider({
    super.key,
    required this.atDur,
  });

  final double atDur;

  @override
  Widget build(BuildContext context) {
    var player = Provider.of<MioPlayerState>(context);
    return Row(
      mainAxisAlignment: MainAxisAlignment.center,
      children: [
        const Icon(Icons.volume_up),
        Slider(
            value: atDur,
            min: 0.0,
            max: 1.0,
            onChanged: (newVal) {
              final x = player.mediaItem.value;
              final Duration dur;
              if (x?.duration == null) {
                dur = const Duration();
              } else {
                dur = Duration(milliseconds: (newVal * x!.duration!.inMilliseconds).toInt());
              }
              player.seek(dur);
            }),
      ],
    );
  }
}
