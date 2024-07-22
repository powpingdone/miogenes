import "package:json_annotation/json_annotation.dart";
import "package:uuid/uuid.dart";

part 'protocol.g.dart';

// String to Uuid converter
class UuidSerde extends JsonConverter<UuidValue, String> {
  const UuidSerde();

  @override
  UuidValue fromJson(String json) {
    return UuidValue.withValidation(json);
  }

  @override
  String toJson(UuidValue object) {
    return object.toString();
  }
}

@JsonSerializable(fieldRename: FieldRename.snake)
@UuidSerde()
class Vers {
  // at protocol/lib.rs
  final UuidValue specialKey0, specialKey1;
  final int major, minor, patch;

  Vers(
      {required this.specialKey0,
      required this.specialKey1,
      required this.major,
      required this.minor,
      required this.patch});

  factory Vers.fromJson(Map<String, dynamic> json) => _$VersFromJson(json);
}

@JsonSerializable(fieldRename: FieldRename.snake)
@UuidSerde()
class Albums {
  final List<UuidValue> album;

  Albums({required this.album});

  factory Albums.fromJson(Map<String, dynamic> json) => _$AlbumsFromJson(json);
}
