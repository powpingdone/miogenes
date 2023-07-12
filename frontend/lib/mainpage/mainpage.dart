import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:flutter_speed_dial/flutter_speed_dial.dart';
import 'package:flutter_spinkit/flutter_spinkit.dart';
import 'package:frontend/ffi.dart';
import 'package:frontend/main.dart';
import 'package:frontend/mainpage/folderview.dart';
import 'package:provider/provider.dart';
import 'package:uuid/uuid.dart';

import 'upload.dart';

class MainNav extends StatefulWidget {
  const MainNav({super.key});

  @override
  State<MainNav> createState() => _MainNavState();
}

class _MainNavState extends State<MainNav> {
  var _pageIndex = 0;
  final List<String> _commonExts = [
    // lossless
    "wav", "flac", "alac",
    // typical lossy
    "mp3", "ogg", "aac", "opus", "m4a"
  ];

  // Albums
  Future<Albums>? albums;

  // Upload tasks
  List<UploadTaskStateHolder> tasks = [];
  bool folderSearchActive = false;

  @override
  Widget build(BuildContext context) {
    final mtl = Provider.of<MioTopLevel>(context);
    var mioState = mtl.mioClient;
    albums ??= mioState.getAlbums();

    // page selection
    Widget page;
    switch (_pageIndex) {
      case 0:
        page = MainPage(albums: albums);
        break;
      case 1:
        page = UploadPage(
          tasks: tasks,
        );
        break;
      default:
        throw UnimplementedError("page $_pageIndex is not implemented");
    }

    // dynamic adding of the fab children if future is not finished
    List<SpeedDialChild> fabChildren = [
      SpeedDialChild(
        child: const Icon(Icons.audiotrack),
        label: "Upload Individual Files",
        // individual file upload
        onTap: () async {
          var navFut = Navigator.of(context);
          var files = await FilePicker.platform.pickFiles(
              allowMultiple: true,
              type: FileType.custom,
              allowedExtensions: _commonExts);
          if (files != null) {
            // get path to upload to
            String? serverPath = await navFut.push(MaterialPageRoute(
                builder: (context) => const FolderViewSelectPage()));
            if (serverPath != null) {
              setState(() => tasks.addAll(files.paths
                  // filter out all nulls
                  .where((x) => x != null)
                  .map((x) => UploadTaskStateHolder(
                        serverPath: serverPath,
                        path: x!,
                        mioClient: mioState,
                      ))));
            }
          }
        },
      )
    ];
    // dynamic part
    if (!folderSearchActive) {
      fabChildren.add(SpeedDialChild(
        child: const Icon(Icons.folder),
        label: "Upload Folder",
        // folder upload
        onTap: () async {
          var navFut = Navigator.of(context);
          var folder = await FilePicker.platform.getDirectoryPath();
          if (folder != null) {
            setState(() => folderSearchActive = true);
            var fut = mioState
                .getFilesAtDir(path: folder)
                .whenComplete(() => setState(() {
                      folderSearchActive = false;
                    }));
            String? serverPath = await navFut.push(MaterialPageRoute(
                builder: (context) => const FolderViewSelectPage()));
            if (serverPath != null) {
              var paths = await fut;
              tasks.addAll(paths.map((path) => UploadTaskStateHolder(
                    serverPath: serverPath,
                    path: path,
                    mioClient: mioState,
                    highestLevel: folder,
                  )));
              setState(() {});
            }
          }
        },
      ));
    }
    return Scaffold(
        appBar: AppBar(
          title: switch (_pageIndex) {
            0 => const Text("Albums"),
            1 => const Text("Upload Files"),
            _ => throw UnimplementedError(),
          },
          backgroundColor: Theme.of(context).colorScheme.primary,
        ),
        // FAB for upload
        floatingActionButton: _pageIndex == 1
            ? SpeedDial(
                children: fabChildren,
                icon: Icons.upload,
              )
            : null,
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
                    icon: Icon(Icons.upload_file), label: Text("Upload files"))
              ],
              selectedIndex: 0,
              onDestinationSelected: (value) =>
                  setState(() => _pageIndex = value),
            )),
            Expanded(child: page),
          ],
        ));
  }
}

class MainPage extends StatelessWidget {
  const MainPage({
    super.key,
    this.albums,
  });

  final Future<Albums>? albums;

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
            return SpinKitWanderingCubes(
              color: Theme.of(context).colorScheme.primary,
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
