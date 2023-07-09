import "package:flutter/material.dart";

class UploadToWhereOverlayPage extends StatelessWidget {
  const UploadToWhereOverlayPage({super.key});

  @override
  Widget build(BuildContext context) {
    // TODO: implement build
    throw UnimplementedError();
  }
}

class UploadPage extends StatelessWidget {
  const UploadPage({
    super.key,
    required this.tasks,
  });

  final List<UploadTask> tasks;

  @override
  Widget build(BuildContext context) {
    return ListView(
      children: tasks,
    );
  }
}

class UploadTask extends StatefulWidget {
  const UploadTask({super.key, required this.rootLevel, required this.path});

  // path to the file to upload
  final String path;
  // parent of path to file, used to seperate the folder system
  final String rootLevel;

  @override
  State<UploadTask> createState() => _UploadTaskState();
}

class _UploadTaskState extends State<UploadTask> {
  bool isFinished = false;

  @override
  Widget build(BuildContext context) {
    return Container();
  }
}
