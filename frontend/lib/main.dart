import 'package:flutter/material.dart';
import 'package:http/http.dart' as http;

void main() {
  runApp(const MiogenesFrontend());
}

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
        home: const LoginHandler(),
        routes: <String, WidgetBuilder>{
          "/login": (context) => const LoginPage(),
        });
  }
}

class LoginPage extends StatefulWidget {
  const LoginPage({super.key});

  @override
  State<LoginPage> createState() => _LoginPageState();
}

class _LoginPageState extends State<LoginPage> {
  late TextEditingController _urlField;
  bool checkingUrl = false;

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
      appBar: AppBar(),
      body: Center(
        child: Column(
          children: [
            TextField(
              controller: _urlField,
              onSubmitted: _checkUrl,
            ),
            ElevatedButton(
                onPressed: () async => _checkUrl(_urlField.text),
                child: const Text("Check URL")),
            checkingUrl
                ? const CircularProgressIndicator.adaptive()
                : Container(),
          ],
        ),
      ),
    );
  }

  Future<bool> _checkUrl(String url) async {
    bool ret;
    setState(() {
      checkingUrl = true;
    });
    
    // TODO: checkurl
    http.Response x = await http.get(Uri.parse(url));

    setState(() {
      checkingUrl = false;
    });
    return ret;
  }
}

class LoginHandler extends StatelessWidget {
  const LoginHandler({super.key});

  @override
  Widget build(BuildContext context) {
    // TODO: check for (working) cached login creds
    Navigator.pushReplacementNamed(context, "/login");
    return const CircularProgressIndicator.adaptive();
  }
}
