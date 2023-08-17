import 'package:flutter/material.dart';
import 'package:flutter_spinkit/flutter_spinkit.dart';
import 'package:frontend/signup.dart';
import 'package:provider/provider.dart';
import 'ffi.dart';
import 'main.dart';
import 'mainpage/toplevel.dart';

class LoginBaseUrl extends StatefulWidget {
  const LoginBaseUrl({super.key});

  @override
  State<LoginBaseUrl> createState() => _LoginBaseUrlState();
}

class _LoginBaseUrlState extends State<LoginBaseUrl>
    with TickerProviderStateMixin {
  late TextEditingController _baseUrlController;

  Future<void>? isValidUrl;

  @override
  void initState() {
    super.initState();
    _baseUrlController = TextEditingController();
  }

  @override
  void dispose() {
    _baseUrlController.dispose();
    super.dispose();
  }

  void setTask(MioClient mioState) {
    setState(() {
      // correct url
      // this is not done in rust, because these corrections need to be visible to the user
      String url = _baseUrlController.text;

      // make sure that the url is prepended with a https://
      if (!url.startsWith(RegExp(r"https?:\/\/"))) {
        url = "https://$url";
      }
      // remove trailing slashes
      while (url.endsWith("/")) {
        url = url.substring(0, url.length - 1);
      }
      isValidUrl = mioState.testSetUrl(url: url);

      // notify listeners
      _baseUrlController.text = url;
    });
  }

  @override
  Widget build(BuildContext context) {
    final mtl = Provider.of<MioTopLevel>(context);
    var mioState = mtl.mioClient;

    var url = mioState.getUrl();
    if (url.isNotEmpty) {
      _baseUrlController.text = url;
    }

    return Scaffold(
        appBar: AppBar(
          title: const Text("Login"),
          backgroundColor: Theme.of(context).colorScheme.primary,
        ),
        body: Padding(
          padding: const EdgeInsets.all(8.0),
          child: Center(
            child: ListView(
              shrinkWrap: true,
              children: [
                Column(
                  children: [
                    Padding(
                      padding: const EdgeInsets.all(8.0),
                      child: SizedBox(
                        width: 300,
                        child: TextField(
                          controller: _baseUrlController,
                          onSubmitted: (_) => setTask(mioState),
                          decoration: const InputDecoration(
                              border: OutlineInputBorder(),
                              labelText: "Base Url"),
                        ),
                      ),
                    ),
                    Padding(
                      padding: const EdgeInsets.all(8.0),
                      child: ElevatedButton(
                          onPressed: () => setTask(mioState),
                          child: const Text("Check Url")),
                    ),
                  ],
                ),
                // Check if server exists
                FutureBuilder(
                  future: isValidUrl,
                  builder: (context, snapshot) {
                    if (snapshot.hasError) {
                      return Text("Invalid url: ${extractMsg(snapshot.error)}");
                    } else if (snapshot.connectionState ==
                        ConnectionState.done) {
                      // build ui for login
                      return const LoginCreds();
                    } else if (isValidUrl == null) {
                      return Container();
                    } else {
                      // show checking
                      return SpinKitWanderingCubes(
                        color: Theme.of(context).colorScheme.primary,
                      );
                    }
                  },
                ),
              ],
            ),
          ),
        ));
  }
}

class LoginCreds extends StatefulWidget {
  const LoginCreds({super.key});

  @override
  State<StatefulWidget> createState() => _LoginCredsState();
}

class _LoginCredsState extends State<LoginCreds> {
  late TextEditingController _usernameController, _passwordController;

  Future<void>? loginCall;

  @override
  void initState() {
    super.initState();
    _usernameController = TextEditingController();
    _passwordController = TextEditingController();
  }

  @override
  void dispose() {
    _usernameController.dispose();
    _passwordController.dispose();
    super.dispose();
  }

  void setTask(MioClient mioState) {
    setState(() {
      loginCall = attemptLogin(mioState);
    });
  }

  Future<void> attemptLogin(MioClient mioState) async {
    var nav = Navigator.of(context);
    await mioState.attemptLogin(
      username: _usernameController.text,
      password: _passwordController.text,
    );
    nav.pushReplacement(
        MaterialPageRoute(builder: (context) => const MainNav()));
  }

  @override
  Widget build(BuildContext context) {
    final mtl = Provider.of<MioTopLevel>(context);
    var mioState = mtl.mioClient;

    return Column(children: [
      Padding(
        padding: const EdgeInsets.all(8.0),
        child: SizedBox(
          width: 300,
          child: TextField(
            controller: _usernameController,
            decoration: const InputDecoration(
                border: OutlineInputBorder(), labelText: "Username"),
          ),
        ),
      ),
      Padding(
        padding: const EdgeInsets.all(8.0),
        child: SizedBox(
          width: 300,
          child: TextField(
            controller: _passwordController,
            obscureText: true,
            decoration: const InputDecoration(
                border: OutlineInputBorder(), labelText: "Password"),
          ),
        ),
      ),
      Row(mainAxisAlignment: MainAxisAlignment.center, children: [
        Padding(
          padding: const EdgeInsets.all(8.0),
          child: ElevatedButton(
              onPressed: () => Navigator.of(context).pushReplacement(
                  MaterialPageRoute(builder: (context) => const SignupPage())),
              child: const Text("Sign Up")),
        ),
        Padding(
          padding: const EdgeInsets.all(8.0),
          child: ElevatedButton(
            onPressed: () => setTask(mioState),
            child: const Text("Sign In"),
          ),
        ),
      ]),
      FutureBuilder(
          future: loginCall,
          builder: (context, snapshot) {
            if (snapshot.hasError) {
              return Text("Could not login: ${extractMsg(snapshot.error)}");
            } else if (snapshot.connectionState == ConnectionState.done) {
              // switch to mainpage
              return Container();
            } else if (loginCall == null) {
              return Container();
            } else {
              // show checking
              return SpinKitWanderingCubes(
                color: Theme.of(context).colorScheme.primary,
              );
            }
          })
    ]);
  }
}
