import "package:flutter/material.dart";
import "package:frontend/ffi.dart";

class UploadPage extends StatelessWidget {
  const UploadPage({
    super.key,
    required this.tasks,
  });

  final List<UploadTaskStateHolder> tasks;

  @override
  Widget build(BuildContext context) {
    return ListView(
      children: [for (var state in tasks) UploadTask(self: state)],
    );
  }
}

class UploadTaskStateHolder {
  const UploadTaskStateHolder(
      {required this.serverPath,
      required this.path,
      required this.uploadFuture});

  // path to the file to upload
  final String path;
  // parent of path to file, used to seperate the folder system
  final String serverPath;
  // the actual future
  final Future<UploadReturn> uploadFuture;
}

class UploadTask extends StatefulWidget {
  const UploadTask({super.key, required this.self});

  final UploadTaskStateHolder self;

  @override
  State<UploadTask> createState() => _UploadTaskState();
}

class _UploadTaskState extends State<UploadTask> with TickerProviderStateMixin {
  late AnimationController _spinner;

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
    super.dispose();
    _spinner.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return FutureBuilder(
        future: widget.self.uploadFuture,
        builder: (context, snapshot) {
          Widget msg;
          if (snapshot.hasData) {
            msg = const Text("Uploaded.");
          } else if (snapshot.hasError) {
            msg = Text("Error encountered: ${extractMsg(snapshot.error)}");
          } else {
            _spinner.forward();
            msg = Row(
              children: [
                CircularProgressIndicator(
                  value: _spinner.value,
                ),
                const Text("Uploading...")
              ],
            );
          }
          return Column(
            children: [Text(widget.self.path), msg],
          );
        });
  }
}
