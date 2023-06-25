import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import 'ffi.dart';
import 'main.dart';

class LoginBaseUrl extends StatefulWidget {
  const LoginBaseUrl({super.key});

  @override
  State<LoginBaseUrl> createState() => _LoginBaseUrlState();
}

class _LoginBaseUrlState extends State<LoginBaseUrl>
    with TickerProviderStateMixin {
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
    super.dispose();
    
    _baseUrlController.dispose();
    _spinner.dispose();
  }

  void setTask(MioClient mioState) {
    setState(() {
      // correct url
      // this is not done in rust, because these corrections need to be visible to the user
      String url = _baseUrlController.text;

      // make sure that the url is prepended with a https://
      if (!url.startsWith(RegExp(r"https?:\/\/"))) {
        url = "https://${url}";
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

    return Column(
      children: [
        TextField(
          controller: _baseUrlController,
          onSubmitted: (_) => setTask(mioState),
          decoration: const InputDecoration(
              border: OutlineInputBorder(), labelText: "Base Url"),
        ),
        // Check if server exists
        FutureBuilder(
          future: isValidUrl,
          builder: (context, snapshot) {
            _spinner.stop();
            if (snapshot.hasError) {
              return Text("Invalid url: ${extractMsg(snapshot.error)}");
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
              onPressed: () => setTask(mioState),
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

class _LoginCredsState extends State<LoginCreds>
    with TickerProviderStateMixin, ChangeNotifier {
  late TextEditingController _usernameController, _passwordController;
  late AnimationController _spinner;

  Future<void>? loginCall;

  @override
  void initState() {
    super.initState();
    _usernameController = TextEditingController();
    _passwordController = TextEditingController();
    _spinner = AnimationController(
        vsync: this, duration: const Duration(seconds: 2, milliseconds: 500))
      ..addListener(() {
        setState(() {});
      })
      ..repeat();
  }

  @override
  void dispose() {
    super.dispose();
    _usernameController.dispose();
    _passwordController.dispose();
    _spinner.dispose();
  }

  void setTask(MioClient mioState) {
    setState(() {
      loginCall = mioState.attemptLogin(
        username: _usernameController.text,
        password: _passwordController.text,
      );
    });
  }

  @override
  Widget build(BuildContext context) {
    final mtl = Provider.of<MioTopLevel>(context);
    var mioState = mtl.mioClient;

    return Column(children: [
      TextField(
        controller: _usernameController,
        decoration: const InputDecoration(
            border: OutlineInputBorder(), labelText: "Username"),
      ),
      TextField(
        controller: _passwordController,
        obscureText: true,
        decoration: const InputDecoration(
            border: OutlineInputBorder(), labelText: "Password"),
      ),
      Row(children: [
        ElevatedButton(
            onPressed: () => mtl.viewport = CurrentViewport.signup,
            child: const Text("Sign Up")),
        ElevatedButton(
          onPressed: () => setTask(mioState),
          child: const Text("Sign In"),
        ),
      ]),
      FutureBuilder(builder: (context, snapshot) {
        _spinner.stop();
        if (snapshot.hasError) {
          return Text("Could not login: ${extractMsg(snapshot.error)}");
        } else if (snapshot.connectionState == ConnectionState.done) {
          // switch to mainpage
          mtl.viewport = CurrentViewport.mainpage;
          return Container();
        } else if (loginCall == null) {
          return Container();
        } else {
          // show checking
          _spinner.forward();
          return CircularProgressIndicator(
            value: _spinner.value,
          );
        }
      })
    ]);
  }
}
