import 'dart:convert';
import 'dart:typed_data';
import 'package:frontend/main.dart';
import 'package:frontend/protocol.dart';
import 'package:http/http.dart' as http;
import 'package:uuid/uuid.dart';

// error class for any specific server errors
class MioError implements Exception {
  late int _statusCode;
  late String _message;

  MioError({required http.Response resp}) {
    _message = jsonDecode(resp.body)["error"] as String;
    _statusCode = resp.statusCode;
  }

  MioError.custom({required int statusCode, required String message})
      : _message = message,
        _statusCode = statusCode;

  get statusCode => _statusCode;
  get message => _message;

  // wrapper for checking on error
  static http.Response check(http.Response resp) {
    if (resp.statusCode != 200) {
      throw MioError(resp: resp);
    }
    return resp;
  }

  @override
  String toString() {
    return message;
  }
}

// Static class for all the calls to the server
class MioRPC {
  MioRPC._();

  // test uri for server, and then set it if it's good. throw otherwise.
  static Future<void> testSetUri(Uri url) async {
    Vers resp;
    try {
      http.Response httpResp = await http.get(pushUri(url, path: "/ver"));
      resp = Vers.fromJson(jsonDecode(httpResp.body));
    } catch (e) {
      throw "Failed to connect to server: $e";
    }
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

  // login to server
  static Future<void> login(
      {required String username, required String password}) async {
    http.Response httpResp = MioError.check(await _nsClient.get("/user/login",
        authString: "Basic ${base64.encode('$username:$password'.codeUnits)}"));
    userToken = httpResp.body;
  }

  // do signup. signup always returns a token
  static Future<void> signup(
      {required String username, required String password}) async {
    http.Response httpResp = MioError.check(await _nsClient.get("/user/signup",
        authString: "Basic ${base64.encode('$username:$password'.codeUnits)}"));
    userToken = httpResp.body;
  }

  static Future<Albums> getAlbums() async {
    http.Response httpResp =
        MioError.check(await _nsClient.get("/load/albums"));
    return Albums.fromJson(jsonDecode(httpResp.body));
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
    String? authString,
  }) async =>
      _send("DELETE", path, query: query, body: body, authString: authString);

  Future<http.Response> get(
    String path, {
    Map<String, dynamic>? query,
    String? authString,
  }) async =>
      _send("GET", path, query: query, authString: authString);

  Future<http.Response> head(
    String path, {
    Map<String, dynamic>? query,
    String? authString,
  }) async =>
      _send("HEAD", path, query: query, authString: authString);

  Future<http.Response> patch(
    String path, {
    Map<String, dynamic>? query,
    Uint8List? body,
    String? authString,
  }) async =>
      _send("PATCH", path, query: query, body: body, authString: authString);

  Future<http.Response> post(
    String path, {
    Map<String, dynamic>? query,
    Uint8List? body,
    String? authString,
  }) async =>
      _send("POST", path, query: query, body: body, authString: authString);

  Future<http.Response> put(
    String path, {
    Map<String, dynamic>? query,
    Uint8List? body,
    String? authString,
  }) async =>
      _send("PUT", path, query: query, body: body, authString: authString);

  // non-streaming send request function. this expects _serverUri to be set and _userToken to be set on null authString
  Future<http.Response> _send(
    String method,
    String path, {
    String? authString,
    Map<String, dynamic>? query,
    Uint8List? body,
  }) async {
    // do the request
    Uri uri = pushUri(serverUri!, path: path, query: query);
    http.Request req = http.Request(method, uri);
    req.headers["Authorization"] = authString ?? "Bearer $userToken";
    req.bodyBytes = body ?? Uint8List(0);
    return http.Response.fromStream(await req.send());
  }
}

// append to server uri. maybe there should have been a "append/overwrite" constructor
Uri pushUri(
  Uri orig, {
  String path = "",
  Map<String, dynamic>? query,
}) {
  return Uri(
    scheme: orig.scheme,
    userInfo: orig.userInfo,
    host: orig.host,
    port: orig.port,
    path: orig.path + path,
    queryParameters: query,
  );
}
