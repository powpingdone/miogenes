import 'package:flutter/material.dart';
import 'package:frontend/login.dart';
import 'package:frontend/mainpage/mainpage.dart';
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
  late AnimationController _spinner;

  Future<void>? signupCall;

  @override
  void initState() {
    super.initState();
    _usernameController = TextEditingController();
    _passwordController = TextEditingController();
    _password2Controller = TextEditingController();
    _spinner = AnimationController(
        vsync: this, duration: const Duration(seconds: 2, milliseconds: 500))
      ..addListener(() {
        setState(() {});
      });
    _spinner.repeat(reverse: true);
  }

  @override
  void dispose() {
    _usernameController.dispose();
    _passwordController.dispose();
    _password2Controller.dispose();
    _spinner.dispose();
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

    return Column(children: [
      TextField(
        controller: _usernameController,
        decoration: const InputDecoration(
            border: OutlineInputBorder(), labelText: "Username"),
        onSubmitted: (_) => setTask(mioState),
      ),
      TextField(
        controller: _passwordController,
        obscureText: true,
        decoration: const InputDecoration(
            border: OutlineInputBorder(), labelText: "Password"),
        onSubmitted: (_) => setTask(mioState),
      ),
      TextField(
        controller: _password2Controller,
        obscureText: true,
        decoration: const InputDecoration(
            border: OutlineInputBorder(), labelText: "Repeat Password"),
        onSubmitted: (_) => setTask(mioState),
      ),
      Row(
        children: [
          ElevatedButton(
              onPressed: () => Navigator.of(context).pushReplacement(
                  MaterialPageRoute(
                      builder: (context) => const LoginBaseUrl())),
              child: const Text("Back To Login")),
          ElevatedButton(
              onPressed: () => setTask(mioState), child: const Text("Sign Up"))
        ],
      ),
      FutureBuilder(
          future: signupCall,
          builder: ((context, snapshot) {
            _spinner.stop();
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
              _spinner.forward();
              return CircularProgressIndicator(
                value: _spinner.value,
              );
            }
          }))
    ]);
  }
}
