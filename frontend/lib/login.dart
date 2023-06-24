import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import 'main.dart';

class LoginPage extends StatefulWidget {
  const LoginPage({super.key});

  @override
  State<LoginPage> createState() => _LoginPageState();
}

class _LoginPageState extends State<LoginPage> with TickerProviderStateMixin {
  late TextEditingController _baseUrlController;
  late AnimationController _spinner;

  Future<void>? isValidUrl;

  @override
  void initState() {
    super.initState();
    _baseUrlController = TextEditingController();
    _spinner = AnimationController(
        vsync: this, duration: const Duration(seconds: 2, milliseconds: 500))
      ..addListener(() {
        setState(() {});
      })
      ..repeat();
  }

  @override
  void dispose() {
    _baseUrlController.dispose();
    _spinner.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final mtl = Provider.of<MioTopLevel>(context);
    var mioState = mtl.mioClient;

    return Column(
      children: [
        TextField(
          controller: _baseUrlController,
          onSubmitted: (url) {
            setState(() {
              isValidUrl = mioState.testSetUrl(url: url);
            });
          },
          decoration: const InputDecoration(
              border: OutlineInputBorder(), labelText: "Base Url"),
        ),
        // Check if server exists
        FutureBuilder(
          future: isValidUrl,
          builder: (context, snapshot) {
            _spinner.stop();
            if (snapshot.hasError) {
              return Text("Invalid url: ${snapshot.error.toString()}");
            } else if (snapshot.connectionState == ConnectionState.done) {
              // build ui for login
              return const LoginCreds();
            } else if (isValidUrl == null) {
              return Container();
            } else {
              // show checking
              _spinner.forward();
              return CircularProgressIndicator(
                value: _spinner.value,
              );
            }
          },
        ),
        Row(children: [
          ElevatedButton(
              onPressed: () {
                setState(() {
                  isValidUrl =
                      mioState.testSetUrl(url: _baseUrlController.text);
                });
              },
              child: const Text("Check Url")),
        ])
      ],
    );
  }
}

class LoginCreds extends StatefulWidget {
  const LoginCreds({super.key});

  @override
  State<StatefulWidget> createState() => _LoginCredsState();
}

class _LoginCredsState extends State<LoginCreds> {
  late TextEditingController _usernameController, _passwordController;

  @override
  void initState() {
    super.initState();
    _usernameController = TextEditingController();
    _passwordController = TextEditingController();
  }

  @override
  void dispose() {
    super.dispose();
    _usernameController.dispose();
    _passwordController.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return Column(children: [
      TextField(
        controller: _passwordController,
        decoration: const InputDecoration(
            border: OutlineInputBorder(), labelText: "Username"),
      ),
      TextField(
        controller: _passwordController,
        obscureText: true,
        decoration: const InputDecoration(
            border: OutlineInputBorder(), labelText: "Password"),
      )
    ]);
  }
}
