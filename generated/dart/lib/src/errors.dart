class InterfaceException implements Exception {
  const InterfaceException(this.code);
  final String code;

  @override
  String toString() => 'InterfaceException($code)';
}

