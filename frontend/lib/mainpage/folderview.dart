import "package:flutter/material.dart";
import "package:flutter_spinkit/flutter_spinkit.dart";
import "package:frontend/ffi.dart";
import "package:frontend/main.dart";
import "package:provider/provider.dart";

class FolderViewSelectPage extends StatefulWidget {
  const FolderViewSelectPage({super.key});

  @override
  State<FolderViewSelectPage> createState() => _FolderViewSelectPageState();
}

class _FolderViewSelectPageState extends State<FolderViewSelectPage> {
  Future<List<FakeMapItem>>? folderFuture;
  List<String> currPath = [];
  Map<String, StrMapContainer?>? tree;

  List<String> _navigatePath() {
    // given the current nav state, return the folders at that nav
    Map<String, StrMapContainer?> currDir = tree!;
    for (String node in currPath) {
      var select = currDir[node];
      // on none/empty dir, bail
      if (select == null) {
        return [];
      } else {
        currDir = select.next;
      }
    }
    return currDir.keys.toList();
  }

  @override
  Widget build(BuildContext context) {
    final mtl = Provider.of<MioTopLevel>(context);
    var mioState = mtl.mioClient;
    folderFuture ??= mioState.getFolders();

    return FutureBuilder(
        future: folderFuture,
        builder: (context, snapshot) {
          if (snapshot.hasData) {
            tree ??= fakeMapConv(snapshot.data!);
            return Column(
              children: [
                Expanded(
                    child: ListView(
                  children: [
                    for (String folder in _navigatePath())
                      TextButton(
                        onPressed: () => setState(() => currPath.add(folder)),
                        child: Row(
                            children: [const Icon(Icons.folder), Text(folder)]),
                      )
                  ],
                )),
                ButtonBar(children: [
                  // Choose directory
                  ElevatedButton(
                      onPressed: () =>
                          Navigator.of(context).pop(currPath.join("/")),
                      child: const Icon(Icons.upload_file)),
                  // create new folder
                  ElevatedButton(
                      onPressed: () => throw UnimplementedError(),
                      child: const Icon(Icons.create_new_folder)),
                  // go up a directory
                  ElevatedButton(
                      onPressed: () => setState(() => currPath.removeLast()),
                      child: const Icon(Icons.arrow_back))
                  // cancel
                  ,
                  ElevatedButton(
                      onPressed: () => Navigator.of(context).pop(),
                      child: const Icon(Icons.cancel))
                ])
              ],
            );
          } else if (snapshot.hasError) {
            return Text(
                "Failed to contact server: ${extractMsg(snapshot.error)}");
          } else {
            return SpinKitWanderingCubes(
              color: Theme.of(context).colorScheme.primary,
            );
          }
        });
  }
}
