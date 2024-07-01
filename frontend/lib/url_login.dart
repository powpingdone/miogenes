import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:frontend/main.dart';
import 'package:frontend/protocol.dart';
import 'package:frontend/server_rpc.dart';
import 'package:http/http.dart' as http;
import 'package:uuid/uuid.dart';

class ServerUrlPage extends StatefulWidget {
  const ServerUrlPage({super.key});

  @override
  State<ServerUrlPage> createState() => _ServerUrlPageState();
}

class _ServerUrlPageState extends State<ServerUrlPage> {
  late TextEditingController _urlField;
  bool _checkingUrl = false;
  String? _errorText;

  @override
  void initState() {
    super.initState();
    _urlField = TextEditingController();
  }

  @override
  void dispose() {
    _urlField.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text("Server URL")),
      body: Center(
        child: Column(
          children: [
            TextField(
              controller: _urlField,
              onSubmitted: _checkUrl,
              decoration: InputDecoration(
                  border: const OutlineInputBorder(),
                  labelText: "ServerUrl",
                  errorText: _errorText),
            ),
            ElevatedButton(
                onPressed: () async => _checkUrl(_urlField.text),
                child: const Text("Check URL")),
            _checkingUrl
                ? const CircularProgressIndicator.adaptive()
                : Container(),
          ],
        ),
      ),
    );
  }

  Future<void> _checkUrl(String url) async {
    final nav = Navigator.of(context);
    setState(() {
      _checkingUrl = true;
    });
    // test and set uri
    try {
      await MioRPC.testSetUri(Uri.parse(url));
    } catch (e) {
      // didn't work
      setState(() {
        _errorText = e.toString();
        _checkingUrl = false;
      });
      return;
    }

    // it did tho, let's login
    setState(() {
      _errorText = "";
      _checkingUrl = false;
    });
    nav.push(MaterialPageRoute(builder: (context) => const LoginPage()));
  }
}

class LoginPage extends StatefulWidget {
  const LoginPage({super.key});

  @override
  State<LoginPage> createState() => _LoginPageState();
}

class _LoginPageState extends State<LoginPage> {
  @override
  Widget build(BuildContext context) {
    // TODO: implement build
    throw UnimplementedError();
  }
}
