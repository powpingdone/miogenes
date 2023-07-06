import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:flutter_speed_dial/flutter_speed_dial.dart';
import 'package:frontend/ffi.dart';
import 'package:frontend/main.dart';
import 'package:provider/provider.dart';
import 'package:uuid/uuid.dart';

import 'upload.dart';

class MainNav extends StatefulWidget {
  const MainNav({super.key});

  @override
  State<MainNav> createState() => _MainNavState();
}

class _MainNavState extends State<MainNav> with TickerProviderStateMixin {
  late AnimationController _spinner;
  var pageIndex = 0;
  final List<String> _commonExts = [
    // lossless
    "wav", "flac", "alac",
    // typical lossy
    "mp3", "ogg", "aac", "opus", "m4a"
  ];

  // Albums
  Future<Albums>? albums;

  // Upload tasks
  List<UploadTask> tasks = const [];
  Future<List<String>>? folderSearch;
  bool folderSearchActive = false, folderSearchTaken = true;

  @override
  void initState() {
    super.initState();
    _spinner = AnimationController(
        vsync: this, duration: const Duration(seconds: 2, milliseconds: 500))
      ..addListener(() {
        setState(() {});
      })
      // TODO: repeat is not correct for this
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

    // page selection
    Widget page;
    switch (pageIndex) {
      case 0:
        page = MainPage(albums: albums, spinner: _spinner);
        break;
      case 1:
        page = UploadPage(
          tasks: tasks,
          folderSearch: folderSearch,
        );
        break;
      default:
        throw UnimplementedError("page $pageIndex is not implemented");
    }

    // dynamic adding of the fab children if future is not finished
    List<SpeedDialChild> fabChildren = [
      SpeedDialChild(
        child: const Icon(Icons.audiotrack),
        label: "Upload Individual Files",
        onTap: () => FilePicker.platform
            .pickFiles(
                allowMultiple: true,
                type: FileType.custom,
                allowedExtensions: _commonExts)
            .then((files) {
          if (files != null) {
            // filter out all nulls
            // TODO: UPDATE THIS WITH THE NEW path
            tasks.addAll(files.paths
                .where((x) => x != null)
                .map((x) => UploadTask(rootLevel: "", path: x!)));
          }
        }),
      )
    ];
    // dynamic part
    if (!folderSearchActive) {
      fabChildren.add(SpeedDialChild(
        child: const Icon(Icons.folder),
        label: "Upload Folder",
        onTap: () => FilePicker.platform.getDirectoryPath().then((folder) {
          if (folder != null) {
            folderSearchActive = true;
            folderSearch = mioState
                .getFilesAtDir(path: folder)
                .whenComplete(() => setState(() {
                      folderSearchActive = false;
                      folderSearchTaken = false;
                    }));
          }
        }),
      ));
    }
    return Scaffold(
        // FAB for upload
        floatingActionButton:
            pageIndex == 1 ? SpeedDial(children: fabChildren) : null,
        // nav rail, and child
        body: Row(
          children: [
            SafeArea(
                child: NavigationRail(
              extended: false,
              destinations: const [
                NavigationRailDestination(
                    icon: Icon(Icons.album), label: Text("Album")),
                NavigationRailDestination(
                    icon: Icon(Icons.upload), label: Text("Upload files"))
              ],
              selectedIndex: 0,
              onDestinationSelected: (value) =>
                  setState(() => pageIndex = value),
            )),
            Expanded(
                // poll folder searching future
                child: FutureBuilder(
              future: folderSearch,
              builder: (context, snapshot) {
                if (snapshot.hasData && !folderSearchTaken) {
                  folderSearchTaken = true;
                  tasks.addAll(snapshot.data!
                      // TODO: UPDATE THIS WITH THE NEW path
                      .map((e) => UploadTask(rootLevel: "", path: e)));
                } else if (snapshot.hasError) {
                  // TODO: report errors
                }
                return page;
              },
            )),
          ],
        ));
  }
}

class MainPage extends StatelessWidget {
  const MainPage({
    super.key,
    this.albums,
    required AnimationController spinner,
  }) : _spinner = spinner;

  final Future<Albums>? albums;
  final AnimationController _spinner;

  @override
  Widget build(BuildContext context) {
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
          // TODO: show error and loading image
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
