// The original content is temporarily commented out to allow generating a self-contained demo - feel free to uncomment later.

import 'package:flutter/material.dart';
import 'package:frontend/url_login.dart';
import 'package:frontend/src/rust/frb_generated.dart';

const mioMAJOR = 0;
const mioMINOR = 1;
const mioPATCH = 0;

void main() async {
  await RustLib.init();
  runApp(const MiogenesFrontend());
}

// Root
class MiogenesFrontend extends StatelessWidget {
  const MiogenesFrontend({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Miogenes Client',
      theme: ThemeData(
        colorScheme: ColorScheme.fromSeed(seedColor: Colors.lightBlue),
        useMaterial3: true,
      ),
      home: const StartupHandler(),
    );
  }
}

// Loader. Immediately switches to serverurlpage if cached login credentials do not exist
class StartupHandler extends StatefulWidget {
  const StartupHandler({super.key});

  @override
  State<StartupHandler> createState() => _StartupHandlerState();
}

class _StartupHandlerState extends State<StartupHandler> {
  Future<void>? fut;

  Future<void> loadState(NavigatorState nav) async {
    // TODO: check for (working) cached login creds
    nav.pushReplacement(
        MaterialPageRoute(builder: (context) => const ServerUrlPage()));
  }

  @override
  Widget build(BuildContext context) {
    fut = fut ?? Future(() async => await loadState(Navigator.of(context)));
    return const Scaffold(
        body: SizedBox(child: CircularProgressIndicator.adaptive()));
  }
}
