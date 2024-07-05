import 'package:flutter/material.dart';
import 'package:frontend/main_page.dart';
import 'package:frontend/server_rpc.dart';

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
  late TextEditingController _username, _password;
  bool _checking = false;
  String? _errorText;

  @override
  void initState() {
    super.initState();
    _username = TextEditingController();
    _password = TextEditingController();
  }

  @override
  void dispose() {
    _password.dispose();
    _username.dispose();
    super.dispose();
  }
  
  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text("Login")),
      body: Column(children: [
            TextField(
              controller: _username,
              onSubmitted: (_) async => _login,
              decoration: const InputDecoration(
                  border: OutlineInputBorder(),
                  labelText: "Username",
                  ),
            ),
            TextField(
              controller: _password,
              onSubmitted: (_) async => _login,
              decoration: InputDecoration(
                  border: const OutlineInputBorder(),
                  labelText: "Password",
                  errorText: _errorText),
            ),
            Row(
              children: [
                ElevatedButton(
                    onPressed: _login,
                    child: const Text("Login")),
                ElevatedButton(
                    onPressed: () => Navigator.push(context, MaterialPageRoute(builder: (context) => _SignupPage())),
                    child: const Text("Signup")
                ),
              ],
            ),

            _checking
                ? const CircularProgressIndicator.adaptive()
                : Container(),
      ]),
    );
  }

  Future<void> _login() async {
    final nav = Navigator.of(context);
    setState(() {
      _checking = true;      
    });
    try {
      MioRPC.login(username: _username.text, password: _password.text);
      _errorText = null;
    } catch (e) {
      _errorText = e.toString();
    }
    if(_errorText == null) {
      nav.pushReplacement(MaterialPageRoute(builder: (context) => const MainPage()));
    }
    setState(() {
      _checking = false;      
    });
  }
}

class _SignupPage extends StatefulWidget{
  @override
  State<_SignupPage> createState() => _SignupPageState();
}

class _SignupPageState extends State<_SignupPage> {
  @override
  Widget build(BuildContext context) {
    // TODO: implement build
    throw UnimplementedError();
  }
}
