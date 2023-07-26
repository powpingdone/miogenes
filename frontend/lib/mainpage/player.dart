import "package:flutter/material.dart";
import "package:frontend/ffi.dart";
import "package:frontend/mainpage/toplevel.dart";
import "package:provider/provider.dart";


class Player extends StatefulWidget {
  const Player({super.key});

  @override
  State<Player> createState() => _PlayerState();
}

class _PlayerState extends State<Player> {
  Stream<PStatus>? stream;

  @override
  Widget build(BuildContext context) {
    var player = Provider.of<MioPlayerState>(context).mioPlayer;
    stream ??= player.infoStream();
    return Row(
      children: [
        Container(), // Cover Art
        Container(), // Title, Artist, Album
        Container(), // Play/Pause, Next
        Container(), // Volume Control
      ],
    );
  }
}