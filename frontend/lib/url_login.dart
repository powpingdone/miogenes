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
              autofocus: true,
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
  late FocusNode _fn;
  bool _checking = false;
  String? _errorText;

  @override
  void initState() {
    super.initState();
    _username = TextEditingController();
    _password = TextEditingController();
    _fn = FocusNode();
  }

  @override
  void dispose() {
    _fn.dispose();
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
          autofocus: true,
          controller: _username,
          onSubmitted: (_) => _fn.requestFocus(),
          decoration: const InputDecoration(
            border: OutlineInputBorder(),
            labelText: "Username",
          ),
        ),
        TextField(
          controller: _password,
          onSubmitted: (_) => _login(_username.text, _password.text),
          obscureText: true,
          focusNode: _fn,
          decoration: InputDecoration(
            border: const OutlineInputBorder(),
            labelText: "Password",
            errorText: _errorText,
          ),
        ),
        Row(
          children: [
            ElevatedButton(
              child: const Text("Login"),
              onPressed: () => _login(_username.text, _password.text),
            ),
            ElevatedButton(
              child: const Text("Signup"),
              onPressed: () => Navigator.push(context,
                  MaterialPageRoute(builder: (context) => _SignupPage())),
            ),
          ],
        ),
        _checking ? const CircularProgressIndicator.adaptive() : Container(),
      ]),
    );
  }

  Future<void> _login(String username, String password) async {
    final nav = Navigator.of(context);
    setState(() {
      _checking = true;
    });
    try {
      await MioRPC.login(username: username, password: password);
      _errorText = null;
    } catch (e) {
      _errorText = e.toString();
    }
    if (_errorText == null) {
      nav.pushReplacement(
          MaterialPageRoute(builder: (context) => const MainPageScaffold()));
    }
    setState(() {
      _checking = false;
    });
  }
}

class _SignupPage extends StatefulWidget {
  @override
  State<_SignupPage> createState() => _SignupPageState();
}

class _SignupPageState extends State<_SignupPage> {
  late TextEditingController _username, _password, _chkPass;
  late FocusNode _passFN, _chkPFN;
  bool _checking = false;
  String? _errorText;

  @override
  void initState() {
    super.initState();
    _username = TextEditingController();
    _password = TextEditingController();
    _chkPass = TextEditingController();
    _passFN = FocusNode();
    _chkPFN = FocusNode();
  }

  @override
  void dispose() {
    _passFN.dispose();
    _password.dispose();
    _username.dispose();
    _chkPass.dispose();
    _chkPFN.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text("Login")),
      body: Column(children: [
        TextField(
          autofocus: true,
          controller: _username,
          onSubmitted: (_) => _passFN.requestFocus(),
          decoration: const InputDecoration(
            border: OutlineInputBorder(),
            labelText: "Username",
          ),
        ),
        TextField(
            controller: _password,
            onSubmitted: (_) => _chkPFN.requestFocus(),
            obscureText: true,
            focusNode: _passFN,
            decoration: const InputDecoration(
              border: OutlineInputBorder(),
              labelText: "Password",
            )),
        TextField(
          controller: _chkPass,
          onSubmitted: (_) =>
              _signup(_username.text, _password.text, _chkPass.text),
          obscureText: true,
          focusNode: _chkPFN,
          decoration: InputDecoration(
              border: const OutlineInputBorder(),
              labelText: "Retype Password",
              errorText: _errorText),
        ),
        Row(
          children: [
            ElevatedButton(
                onPressed: () =>
                    _signup(_username.text, _password.text, _chkPass.text),
                child: const Text("Login")),
            ElevatedButton(
                onPressed: () => Navigator.push(context,
                    MaterialPageRoute(builder: (context) => _SignupPage())),
                child: const Text("Signup")),
          ],
        ),
        _checking ? const CircularProgressIndicator.adaptive() : Container(),
      ]),
    );
  }

  Future<void> _signup(String username, String password, String cmpPass) async {
    final nav = Navigator.of(context);
    setState(() {
      _checking = true;
    });
    try {
      if (password != cmpPass) {
        throw "The passwords do not match.";
      }
      await MioRPC.signup(username: username, password: password);
      nav.pushReplacement(
          MaterialPageRoute(builder: (context) => const MainPageScaffold()));
      _errorText = null;
    } catch (e) {
      _errorText = e.toString();
    }
    setState(() {
      _checking = false;
    });
  }
}
