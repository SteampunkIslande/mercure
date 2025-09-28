function isVarTypeOf(_var, _type) {
  try {
    return _var.constructor === _type;
  } catch (ex) {
    return false; //fallback for null or undefined
  }
}
