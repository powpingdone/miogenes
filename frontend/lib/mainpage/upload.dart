import "package:flutter/material.dart";

class UploadPage extends StatefulWidget {
  const UploadPage({
    super.key,
    this.folderSearch,
    required this.tasks,
  });

  final List<UploadTask> tasks;
  final Future<List<String>>? folderSearch;

  @override
  State<UploadPage> createState() => _UploadPageState();
}

class _UploadPageState extends State<UploadPage> {
  @override
  Widget build(BuildContext context) {
    return Container();
  }
}

class UploadTask extends StatefulWidget {
  const UploadTask({
    super.key,
    required this.path,
  });

  final String path;

  @override
  State<UploadTask> createState() => _UploadTaskState();
}

class _UploadTaskState extends State<UploadTask> {
  bool isFinished = false;

  @override
  Widget build(BuildContext context) {
    // TODO: implement build
    throw UnimplementedError();
  }
}
