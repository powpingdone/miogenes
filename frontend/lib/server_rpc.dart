import 'dart:convert';
import 'dart:typed_data';
import 'package:frontend/main.dart';
import 'package:frontend/protocol.dart';
import 'package:http/http.dart' as http;
import 'package:uuid/uuid.dart';

// Class for all the calls to the server
class MioRPC {
  MioRPC._();

  // test uri for server, and then set it if it's good. throw otherwise.
  static Future<void> testSetUri(Uri url) async {
    http.Response httpResp = await http.get(pushUri(url, path: "/ver"));
    Vers resp = Vers.fromJson(jsonDecode(httpResp.body));
    // check if this is "acting" like a miogenes server
    if (!(resp.specialKey0 ==
            UuidValue.fromString("ddf6b403-6a16-4b65-92e0-8342cad3c3e1") &&
        resp.specialKey1 ==
            UuidValue.fromString("b39120cb-f4be-49b5-93ef-9da95610df7d"))) {
      throw "This is not a miogenes server. Looked at the special keys but did not get expected output.";
    }
    // then check if this is "compatible"
    if (!(resp.major == mioMAJOR &&
        resp.minor == mioMINOR &&
        resp.patch >= mioPATCH)) {
      throw "This client/server is outdated and will not connect to the server."
          " Expected $mioMAJOR.$mioMINOR with patch less than or equal to $mioPATCH,"
          " got ${resp.major}.${resp.minor}.${resp.patch} instead.";
    }
    // we're compatible and a miogenes server
    serverUri = url;
  }

  // Setters for internal parameters
  static set serverUri(Uri? serverUri) {
    _nsClient.serverUri = serverUri;
  }

  static set userToken(String? userToken) {
    _nsClient.userToken = userToken;
  }

  // streaming class later
  static final _nsClient = _MiogenesHttpClient();
}

/// !!
/// !! NON-STREAMING HANDLERS. Used for requests that don't stream their output, and are basically just json
/// !!
class _MiogenesHttpClient {
  Uri? serverUri;
  String? userToken;

  // generic handlers for sending non-streaming requests
  Future<http.Response> delete(
    String path, {
    Map<String, dynamic>? query,
    Uint8List? body,
    bool addAuth = true,
  }) async =>
      _send("DELETE", path, query: query, body: body, addAuth: addAuth);

  Future<http.Response> get(
    String path, {
    Map<String, dynamic>? query,
    bool addAuth = true,
  }) async =>
      _send("GET", path, query: query, addAuth: addAuth);

  Future<http.Response> head(
    String path, {
    Map<String, dynamic>? query,
    bool addAuth = true,
  }) async =>
      _send("HEAD", path, query: query, addAuth: addAuth);

  Future<http.Response> patch(
    String path, {
    Map<String, dynamic>? query,
    Uint8List? body,
    bool addAuth = true,
  }) async =>
      _send("PATCH", path, query: query, body: body, addAuth: addAuth);

  Future<http.Response> post(
    String path, {
    Map<String, dynamic>? query,
    Uint8List? body,
    bool addAuth = true,
  }) async =>
      _send("POST", path, query: query, body: body, addAuth: addAuth);

  Future<http.Response> put(
    String path, {
    Map<String, dynamic>? query,
    Uint8List? body,
    bool addAuth = true,
  }) async =>
      _send("PUT", path, query: query, body: body, addAuth: addAuth);

  // non-streaming send request function. this expects _serverUri to be set and _userToken to be set on addAuth
  Future<http.Response> _send(
    String method,
    String path, {
    required bool addAuth,
    Map<String, dynamic>? query,
    Uint8List? body,
  }) async {
    // do the request
    Uri uri = pushUri(serverUri!, path: path, query: query);
    http.Request req = http.Request(method, uri);
    if (addAuth) {
      req.headers["Authorization"] = "Bearer $userToken";
    }
    req.bodyBytes = body ?? Uint8List(0);
    return http.Response.fromStream(await req.send());
  }
}

Uri pushUri(
  Uri orig, {
  String path = "",
  Map<String, dynamic>? query,
}) {
  // append to server uri. maybe there should have been a "append/overwrite" constructor
  return Uri(
    scheme: orig.scheme,
    userInfo: orig.userInfo,
    host: orig.host,
    port: orig.port,
    path: orig.path + path,
    queryParameters: query,
  );
}
