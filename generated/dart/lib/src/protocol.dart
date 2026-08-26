import 'errors.dart';
import 'models.dart';

const protocolVersion = '1';
const schemaRevision = 'flags-2-env-0001';

FlagCatalog parseFlagCatalog(String id, String revision, Map<String, Object?> payload) {
  if (id.trim().isEmpty) {
    throw const InterfaceException('empty_id');
  }
  if (revision.trim().isEmpty) {
    throw const InterfaceException('empty_revision');
  }
  return FlagCatalog(id: id, revision: revision, payload: payload);
}

