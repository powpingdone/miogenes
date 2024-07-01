import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:frontend/url_login.dart';

const mioMAJOR = 0;
const mioMINOR = 1;
const mioPATCH = 0;

void main() {
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
class StartupHandler extends StatelessWidget {
  const StartupHandler({super.key});

  @override
  Widget build(BuildContext context) {
    // TODO: check for (working) cached login creds
    Navigator.pushReplacement(context,
        MaterialPageRoute(builder: (context) => const ServerUrlPage()));
    return const CircularProgressIndicator.adaptive();
  }
}
