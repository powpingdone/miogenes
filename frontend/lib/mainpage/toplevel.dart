import 'dart:async';

import 'package:audio_service/audio_service.dart';
import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:flutter_speed_dial/flutter_speed_dial.dart';
import 'package:frontend/ffi.dart';
import 'package:frontend/main.dart';
import 'package:frontend/mainpage/folderview.dart';
import 'package:frontend/mainpage/player.dart' as ui_player;
import 'package:provider/provider.dart';
import 'package:uuid/uuid.dart';

import 'albums.dart';
import 'upload.dart';

class MainNav extends StatelessWidget {
  const MainNav({super.key});

  @override
  Widget build(BuildContext context) {
    return MultiProvider(providers: [
      ChangeNotifierProvider(create: (_) => MainNavTopLevel()),
      Provider(
          lazy: false,
          create: (context) async {
            final mtl = Provider.of<MioTopLevel>(context, listen: false);
            var mioState = mtl.mioClient;
            // TODO: android
            return await AudioService.init(
              builder: () =>
                  MioPlayerState(api.newPlayer(client: mioState), mioState),
              // config:
            );
          })
    ], builder: (_, __) => const MainNavWidgetPage());
  }
}

// Music audio player state
class MioPlayerState extends BaseAudioHandler with SeekHandler {
  // TODO: android
  MioPlayerState(this._mioPlayer, this._mioClient) {
    _mioStatusStream = _mioPlayer.infoStream();
    _mioStatusListener = _mioStatusStream.listen(_update);
  }
  final MioPlayer _mioPlayer;
  final MioClient _mioClient;
  late Stream<PStatus> _mioStatusStream;
  // ignore: unused_field
  late StreamSubscription<void> _mioStatusListener;

  // internal player stuff
  UuidValue? _curr;
  Track? _track;
  Album? _album;
  Artist? _artist;
  CoverArt? _coverArt;
  BigInt _futureId = BigInt.zero;

  void enqueue(UuidValue id) {
    _mioPlayer.queue(id: id);
  }

  void _update(PStatus status) {
    // setup background task if id is different
    if (_curr != status.currPlaying) {
      _track = null;
      _album = null;
      _artist = null;
      _coverArt = null;
      _curr = status.currPlaying;
      _futureId += BigInt.one;
      if (_curr != null) {
        Future(() async {
          final futureId = _futureId;
          final id = status.currPlaying!;
          final trackMdata = await _mioClient.getTrack(id: id);
          final Future<Album>? albumMdataFuture = trackMdata.album == null
              ? _mioClient.getAlbum(id: trackMdata.album!)
              : null;
          final Future<Artist>? artistMdataFuture = trackMdata.artist == null
              ? _mioClient.getArtist(id: trackMdata.artist!)
              : null;
          final Future<CoverArt>? coverArtMdataFuture =
              trackMdata.coverArt == null
                  ? _mioClient.getCoverArt(id: trackMdata.coverArt!)
                  : null;
          final Album? albumMdata = await albumMdataFuture;
          final Artist? artistMdata = await artistMdataFuture;
          final CoverArt? coverArtMdata = await coverArtMdataFuture;
          if (_futureId == futureId) {
            _track = trackMdata;
            _album = albumMdata;
            _artist = artistMdata;
            _coverArt = coverArtMdata;
          }
        });
      }
    }

// playback state
    playbackState.add(PlaybackState(
        processingState: const {
          null: AudioProcessingState.idle,
          DecoderStatus.Loading: AudioProcessingState.loading,
          DecoderStatus.Buffering: AudioProcessingState.buffering,
          DecoderStatus.Paused: AudioProcessingState.ready,
          DecoderStatus.Playing: AudioProcessingState.ready,
          DecoderStatus.Dead: AudioProcessingState.completed,
        }[status.status]!,
        playing: !(status.status == DecoderStatus.Paused ||
            status.status == DecoderStatus.Dead),
        updatePosition: Duration(
            seconds: status.playbackPosS, milliseconds: status.playbackPosMs),
        controls: [
          MediaControl.rewind,
          if (!(status.status == DecoderStatus.Paused ||
              status.status == DecoderStatus.Dead))
            MediaControl.play
          else
            MediaControl.pause,
          MediaControl.stop,
          MediaControl.fastForward,
        ],
        systemActions: const {
          MediaAction.seek,
          MediaAction.seekForward,
          MediaAction.seekBackward,
        }));

    // media item
    final String title;
    if (_track?.title == null) {
      if (_curr == null) {
        title = "";
      } else {
        title = "Loading...";
      }
    } else {
      title = _track!.title;
    }
    mediaItem.add(MediaItem(
        id: status.currPlaying.toString(),
        title: title,
        album: _album?.title,
        artist: _artist?.name,
        artUri: _coverArt != null
            ? Uri.dataFromBytes(_coverArt!.webmBlob.toList(),
                mimeType: "image/webp")
            : null,
        duration: Duration(
            seconds: status.playbackLenS, milliseconds: status.playbackLenMs)));
  }

  Future<void> toggle() => _mioPlayer.toggle();

  @override
  Future<void> play() => _mioPlayer.play();

  @override
  Future<void> pause() => _mioPlayer.pause();

  @override
  Future<void> stop() => _mioPlayer.stop();

  @override
  Future<void> skipToNext() => _mioPlayer.forward();

  @override
  Future<void> skipToPrevious() => _mioPlayer.backward();

  @override
  Future<void> seek(Duration position) =>
      _mioPlayer.seek(ms: position.inMilliseconds);
}

// UI state for post login
class MainNavTopLevel extends ChangeNotifier {
  // albums
  Future<Albums>? _albums;
  Future<Albums>? get albums => _albums;
  set albums(Future<Albums>? albums) {
    bool nullAlbums = _albums == null;
    _albums = albums;
    if (!nullAlbums) {
      notifyListeners();
    }
  }

  // Upload tasks
  final List<UploadTaskStateHolder> _tasks = [];
  List<UploadTaskStateHolder> get tasks => List.unmodifiable(_tasks);

  void cleanup() {
    _tasks.removeWhere((element) => element.finished);
    notifyListeners();
  }

  void addToUploadTasks(Iterable<UploadTaskStateHolder> newTasks) {
    _tasks.addAll(newTasks);
    notifyListeners();
  }
}

class MainNavWidgetPage extends StatefulWidget {
  const MainNavWidgetPage({super.key});

  @override
  State<MainNavWidgetPage> createState() => _MainNavWidgetPageState();
}

class _MainNavWidgetPageState extends State<MainNavWidgetPage> {
  var _pageIndex = 0;

  @override
  Widget build(BuildContext context) {
    final mtl = Provider.of<MioTopLevel>(context);
    var mioState = mtl.mioClient;
    final mainState = Provider.of<MainNavTopLevel>(context);
    const List<String> commonExts = [
      // lossless
      "wav", "flac", "alac",
      // typical lossy
      "mp3", "ogg", "aac", "m4a"
    ];
    mainState.albums ??= mioState.getAlbums();

    // page selection
    Widget page;
    switch (_pageIndex) {
      case 0:
        page = const ui_player.Player();
        break;
      case 1:
        page = const AlbumPage();
        break;
      case 2:
        page = const UploadPage();
        break;
      // TODO: folder track select page
      default:
        throw UnimplementedError("page $_pageIndex is not implemented");
    }

    return Scaffold(
      appBar: AppBar(
        title: Text(
          switch (_pageIndex) {
            0 => "Player",
            1 => "Albums",
            2 => "Upload Files",
            _ => throw UnimplementedError(),
          },
          style: TextStyle(color: Theme.of(context).colorScheme.onPrimary),
        ),
        actions: switch (_pageIndex) {
          2 => [
              SafeArea(
                child: IconButton(
                    onPressed: () => mainState.cleanup(),
                    icon: Icon(
                      Icons.clear_all,
                      color: Theme.of(context).colorScheme.onPrimary,
                    )),
              )
            ],
          _ => const []
        },
        backgroundColor: Theme.of(context).colorScheme.primary,
      ),
      // FAB for upload
      floatingActionButton: _pageIndex != 2
          ? null
          : SpeedDial(
              children: [
                SpeedDialChild(
                  child: const Icon(Icons.audiotrack),
                  label: "Upload Individual Files",
                  // individual file upload
                  onTap: () async {
                    var navFut = Navigator.of(context);
                    var files = await FilePicker.platform.pickFiles(
                        allowMultiple: true,
                        type: FileType.custom,
                        allowedExtensions: commonExts);
                    if (files != null) {
                      // get server path to upload to
                      String? serverPath = await navFut.push(MaterialPageRoute(
                          builder: (context) => const FolderViewSelectPage()));
                      if (serverPath != null) {
                        mainState.addToUploadTasks(files.paths
                            // filter out all nulls
                            .where((x) => x != null)
                            .map((x) => UploadTaskStateHolder(
                                  serverPath: serverPath,
                                  path: x!,
                                  mioClient: mioState,
                                  mainNav: mainState,
                                )));
                      }
                    }
                  },
                ),
                SpeedDialChild(
                  child: const Icon(Icons.folder),
                  label: "Upload Folders",
                  // folder upload
                  onTap: () async {
                    var navFut = Navigator.of(context);
                    // get toplevel to search
                    var folder = await FilePicker.platform.getDirectoryPath();
                    if (folder != null) {
                      var fut = mioState.getFilesAtDir(path: folder);
                      // get server path to upload to
                      String? serverPath = await navFut.push(MaterialPageRoute(
                          builder: (context) => const FolderViewSelectPage()));
                      if (serverPath != null) {
                        var paths = await fut;
                        mainState.addToUploadTasks(
                            paths.map((path) => UploadTaskStateHolder(
                                  serverPath: serverPath,
                                  path: path,
                                  mioClient: mioState,
                                  highestLevel: folder,
                                  mainNav: mainState,
                                )));
                      }
                    }
                  },
                )
              ],
              icon: Icons.upload,
            ),
      // nav rail, and child
      body: SafeArea(
          child: Row(
        children: [
          NavigationRail(
            extended: MediaQuery.of(context).size.width > 500,
            destinations: const [
              NavigationRailDestination(
                  icon: Icon(Icons.music_note), label: Text("Player")),
              NavigationRailDestination(
                  icon: Icon(Icons.album), label: Text("Album")),
              NavigationRailDestination(
                  icon: Icon(Icons.upload_file), label: Text("Upload files"))
            ],
            selectedIndex: _pageIndex,
            onDestinationSelected: (value) =>
                setState(() => _pageIndex = value),
          ),
          Expanded(child: page),
        ],
      )),
    );
  }
}
