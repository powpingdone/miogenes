import "package:flutter/material.dart";
import "package:frontend/ffi.dart";
import 'package:frontend/mainpage/toplevel.dart';
import "package:path/path.dart" as path_util;
import "package:flutter_spinkit/flutter_spinkit.dart";
import "package:provider/provider.dart";
import "package:text_scroll/text_scroll.dart";

class UploadPage extends StatelessWidget {
  const UploadPage({
    super.key,
  });

  @override
  Widget build(BuildContext context) {
    var mainState = Provider.of<MainNavTopLevel>(context);
    if (mainState.tasks.isEmpty) {
      return Container(
        alignment: Alignment.center,
        child: const Text(
            "No files are uploading currently. Select files to upload using the bottom right button."),
      );
    } else {
      return ListView(
        children: [for (var state in mainState.tasks) UploadTask(self: state)],
      );
    }
  }
}

class UploadTaskStateHolder {
  UploadTaskStateHolder(
      {this.highestLevel,
      required this.serverPath,
      required this.path,
      required MioClient mioClient,
      required MainNavTopLevel mainNav}) {
    uploadFuture = _internalFuture(mioClient, mainNav);
  }

  // path to the file to upload
  final String path;
  // the path on the server to upload the file to
  final String serverPath;
  // the highest level on all the paths selected
  final String? highestLevel;
  // when future is finished, cleanup this
  bool finished = false;
  // the actual future
  late Future<UploadReturn> uploadFuture;

  Future<UploadReturn> _internalFuture(
      MioClient mioClient, MainNavTopLevel mainNav) async {
    try {
      // path_util not needed here because this is an internal path
      var serverNodes =
          serverPath.isEmpty ? [] : serverPath.split("/").toList();
      // pickup the rest of the paths extending the path
      if (highestLevel != null) {
        var splitPath = path_util.split(path);
        serverNodes.addAll(splitPath.sublist(
            path_util.split(highestLevel!).length, splitPath.length - 1));
      }

      // create dirs even if they already exist
      // this sucks. honest. but hey, at least I already implemented the
      // locking to make sure that this doesn't do funky stuff serverside.
      for (var x = 0; x < serverNodes.length; x++) {
        // return new path
        String pathTo = serverNodes.sublist(0, x).join("/");
        try {
          await mioClient.makeDir(name: serverNodes[x], path: pathTo);
        } catch (_) {
          // doesn't really matter
        }
      }
      var ret = await mioClient.uploadFile(
          fullpath: path, dir: serverNodes.join("/"));
      mainNav.albums = mioClient.getAlbums();
      return ret;
    } finally {
      finished = true;
    }
  }
}

class UploadTask extends StatelessWidget {
  const UploadTask({super.key, required this.self});

  final UploadTaskStateHolder self;

  @override
  Widget build(BuildContext context) {
    return FutureBuilder(
        future: self.uploadFuture,
        builder: (context, snapshot) {
          Widget msg;
          if (snapshot.hasData) {
            msg = const Text("Uploaded.");
          } else if (snapshot.hasError) {
            msg = Center(
                child:
                    Text("Error encountered: ${extractMsg(snapshot.error)}"));
          } else {
            msg = Row(
              mainAxisAlignment: MainAxisAlignment.center,
              children: [
                Padding(
                  padding: const EdgeInsets.symmetric(horizontal: 8.0),
                  child: SpinKitWanderingCubes(
                    color: Theme.of(context).colorScheme.primary,
                    size: 16.0,
                  ),
                ),
                const Padding(
                  padding: EdgeInsets.symmetric(horizontal: 8.0),
                  child: Text("Uploading..."),
                )
              ],
            );
          }
          return Container(
            padding: const EdgeInsets.fromLTRB(0.0, 16.0, 0.0, 0.0),
            child: Column(
              children: [
                TextScroll(
                  self.path,
                  mode: TextScrollMode.bouncing,
                  style: const TextStyle(fontWeight: FontWeight.bold),
                ),
                msg
              ],
            ),
          );
        });
  }
}
