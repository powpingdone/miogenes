import "package:flutter/material.dart";
import "package:frontend/ffi.dart";
import "package:frontend/main.dart";
import "package:provider/provider.dart";

class FolderViewSelectPage extends StatefulWidget {
  const FolderViewSelectPage({super.key});

  @override
  State<FolderViewSelectPage> createState() => _FolderViewSelectPageState();
}

class _FolderViewSelectPageState extends State<FolderViewSelectPage>
    with TickerProviderStateMixin {
  late AnimationController _spinner;

  Future<List<FakeMapItem>>? folderFuture;
  List<String> currPath = [];
  Map<String, StrMapContainer?>? tree;

  @override
  void initState() {
    super.initState();
    _spinner = AnimationController(
        vsync: this, duration: const Duration(seconds: 2, milliseconds: 500))
      ..addListener(() {
        setState(() {});
      });
    _spinner.repeat(reverse: true);
  }

  @override
  void dispose() {
    _spinner.dispose();
    super.dispose();
  }

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
            _spinner.forward();
            return CircularProgressIndicator(
              value: _spinner.value,
            );
          }
        });
  }
}
