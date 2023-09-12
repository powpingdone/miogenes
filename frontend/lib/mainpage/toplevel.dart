import 'package:audio_service/audio_service.dart';
import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:flutter_speed_dial/flutter_speed_dial.dart';
import 'package:frontend/ffi.dart';
import 'package:frontend/main.dart';
import 'package:frontend/mainpage/folderview.dart';
import 'package:frontend/mainpage/player.dart' as ui_player;
import 'package:provider/provider.dart';

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
          create: (context) {
            final mtl = Provider.of<MioTopLevel>(context, listen: false);
            var mioState = mtl.mioClient;
            return MioPlayerState(api.newPlayer(client: mioState));
          })
    ], builder: (_, __) => const MainNavWidgetPage());
  }
}

// Music audio player state
class MioPlayerState {
  MioPlayerState(mioInternal) {
    AudioService.init(builder: () => MioAudioServiceGlue(mioInternal))
        .then((x) => audioService = x);
  }
  late MioAudioServiceGlue audioService;
}

class MioAudioServiceGlue extends BaseAudioHandler {
  MioAudioServiceGlue(this._mioInternal);
  final MioPlayer _mioInternal;

  // TODO: android

  @override
  Future<void> play() => Future.sync(() => _mioInternal.play());

  @override
  Future<void> pause() => Future.sync(() => _mioInternal.pause());

  @override
  Future<void> stop() => Future.sync(() => _mioInternal.stop());

  @override
  Future<void> skipToNext() => Future.sync(() => _mioInternal.forward());
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
