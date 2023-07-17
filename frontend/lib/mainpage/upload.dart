import "package:flutter/material.dart";
import "package:frontend/ffi.dart";
import 'package:frontend/mainpage/toplevel.dart';
import "package:path/path.dart" as path_util;
import "package:flutter_spinkit/flutter_spinkit.dart";
import "package:provider/provider.dart";

class UploadPage extends StatelessWidget {
  const UploadPage({
    super.key,
  });

  @override
  Widget build(BuildContext context) {
    var mainState = Provider.of<MainNavTopLevel>(context);
    return ListView(
      children: [for (var state in mainState.tasks) UploadTask(self: state)],
    );
  }
}

class UploadTaskStateHolder {
  UploadTaskStateHolder(
      {this.highestLevel,
      required this.serverPath,
      required this.path,
      required MioClient mioClient}) {
    uploadFuture = _internalFuture(mioClient);
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

  Future<UploadReturn> _internalFuture(MioClient mioClient) async {
    // path_util not needed here because this is an internal path
    var serverNodes = serverPath.isEmpty ? [] : serverPath.split("/").toList();
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
    var ret =
        await mioClient.uploadFile(fullpath: path, dir: serverNodes.join("/"));
    finished = true;
    return ret;
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
            msg = Text("Error encountered: ${extractMsg(snapshot.error)}");
          } else {
            msg = Row(
              children: [
                SpinKitWanderingCubes(
                  color: Theme.of(context).colorScheme.primary,
                ),
                const Text("Uploading...")
              ],
            );
          }
          return Column(
            children: [Text(self.path), msg],
          );
        });
  }
}
