// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'protocol.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

Vers _$VersFromJson(Map<String, dynamic> json) => Vers(
      specialKey0: const UuidSerde().fromJson(json['special_key0'] as String),
      specialKey1: const UuidSerde().fromJson(json['special_key1'] as String),
      major: (json['major'] as num).toInt(),
      minor: (json['minor'] as num).toInt(),
      patch: (json['patch'] as num).toInt(),
    );

Map<String, dynamic> _$VersToJson(Vers instance) => <String, dynamic>{
      'special_key0': const UuidSerde().toJson(instance.specialKey0),
      'special_key1': const UuidSerde().toJson(instance.specialKey1),
      'major': instance.major,
      'minor': instance.minor,
      'patch': instance.patch,
    };
