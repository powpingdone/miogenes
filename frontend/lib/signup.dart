import 'package:flutter/material.dart';
import 'package:flutter_spinkit/flutter_spinkit.dart';
import 'package:frontend/login.dart';
import 'package:frontend/mainpage/toplevel.dart';
import 'package:provider/provider.dart';
import 'ffi.dart';
import 'main.dart';

class SignupPage extends StatefulWidget {
  const SignupPage({
    super.key,
  });

  @override
  State<SignupPage> createState() => _SignupPageState();
}

class _SignupPageState extends State<SignupPage> with TickerProviderStateMixin {
  late TextEditingController _usernameController,
      _passwordController,
      _password2Controller;

  Future<void>? signupCall;

  @override
  void initState() {
    super.initState();
    _usernameController = TextEditingController();
    _passwordController = TextEditingController();
    _password2Controller = TextEditingController();
  }

  @override
  void dispose() {
    _usernameController.dispose();
    _passwordController.dispose();
    _password2Controller.dispose();
    super.dispose();
  }

  void setTask(MioClient mioState) {
    setState(() {
      signupCall = callMio(mioState);
    });
  }

  Future<void> callMio(MioClient mioState) async {
    var nav = Navigator.of(context);
    await mioState.attemptSignupAndLogin(
        username: _usernameController.text,
        password: _passwordController.text,
        password2: _password2Controller.text);
    nav.pushReplacement(
        MaterialPageRoute(builder: (context) => const MainNav()));
  }

  @override
  Widget build(BuildContext context) {
    final mtl = Provider.of<MioTopLevel>(context);
    var mioState = mtl.mioClient;

    return Scaffold(
      appBar: AppBar(
        title: Text(
          "Sign Up",
          style: TextStyle(color: Theme.of(context).colorScheme.onPrimary),
        ),
        backgroundColor: Theme.of(context).colorScheme.primary,
      ),
      body: Column(mainAxisAlignment: MainAxisAlignment.center, children: [
        Padding(
          padding: const EdgeInsets.all(8.0),
          child: SizedBox(
            width: 300,
            child: TextField(
              controller: _usernameController,
              decoration: const InputDecoration(
                  border: OutlineInputBorder(), labelText: "Username"),
              onSubmitted: (_) => setTask(mioState),
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
              onSubmitted: (_) => setTask(mioState),
            ),
          ),
        ),
        Padding(
          padding: const EdgeInsets.all(8.0),
          child: SizedBox(
            width: 300,
            child: TextField(
              controller: _password2Controller,
              obscureText: true,
              decoration: const InputDecoration(
                  border: OutlineInputBorder(), labelText: "Repeat Password"),
              onSubmitted: (_) => setTask(mioState),
            ),
          ),
        ),
        Row(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            Padding(
              padding: const EdgeInsets.all(8.0),
              child: ElevatedButton(
                  onPressed: () => Navigator.of(context).pushReplacement(
                      MaterialPageRoute(
                          builder: (context) => const LoginBaseUrl())),
                  child: const Text("Back To Login")),
            ),
            Padding(
              padding: const EdgeInsets.all(8.0),
              child: ElevatedButton(
                  onPressed: () => setTask(mioState),
                  child: const Text("Sign Up")),
            )
          ],
        ),
        FutureBuilder(
            future: signupCall,
            builder: ((context, snapshot) {
              if (snapshot.hasError) {
                return Text(
                    "Could not signup and login: ${extractMsg(snapshot.error)}");
              } else if (snapshot.connectionState == ConnectionState.done) {
                // switch to mainpage
                return Container();
              } else if (signupCall == null) {
                return Container();
              } else {
                // show checking
                return SpinKitWanderingCubes(
                  color: Theme.of(context).colorScheme.primary,
                );
              }
            }))
      ]),
    );
  }
}
