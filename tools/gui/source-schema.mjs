// Only the bounded schema vocabulary emitted by our owned source-page DTO.
// Unknown constraints fail generation instead of silently weakening validation.
function only(schema, keys) {
  for (const key of Object.keys(schema))
    if (
      ![
        "$schema",
        "$defs",
        "title",
        "description",
        "default",
        ...keys,
      ].includes(key)
    )
      throw new Error("Unsupported source DTO constraint: " + key);
}
export function validator(schema, root = schema) {
  // An explicit empty JSON schema denotes opaque diagnostic data. Unknown
  // keywords still fail below; this does not decode a plugin or authority.
  if (
    schema === true ||
    (schema &&
      typeof schema === "object" &&
      !Array.isArray(schema) &&
      Object.keys(schema).every((key) =>
        ["$schema", "$defs", "title", "description", "default"].includes(key),
      ))
  )
    return "z.unknown()";
  if (schema.$ref) {
    only(schema, ["$ref"]);
    const key = schema.$ref.replace(/^#\/\$defs\//, "");
    if (!root.$defs?.[key]) throw new Error("Unknown DTO schema reference");
    return validator(root.$defs[key], root);
  }
  if (schema.const !== undefined) {
    only(schema, ["const", "type"]);
    if (
      typeof schema.const !== "string" ||
      (schema.type !== undefined && schema.type !== "string")
    )
      throw new Error("Only string DTO constants are supported");
    return `z.literal(${JSON.stringify(schema.const)})`;
  }
  if (schema.enum) {
    only(schema, ["enum", "type"]);
    if (
      !schema.enum.length ||
      !schema.enum.every((x) => typeof x === "string") ||
      (schema.type !== undefined && schema.type !== "string")
    )
      throw new Error("Only string DTO enums are supported");
    return `z.enum(${JSON.stringify(schema.enum)})`;
  }
  const variants = schema.anyOf || schema.oneOf;
  if (variants) {
    only(schema, [schema.anyOf ? "anyOf" : "oneOf"]);
    if (!variants.length) throw new Error("Empty DTO union");
    if (schema.oneOf) {
      const literals =
        variants.every((s) => typeof s.const === "string") &&
        new Set(variants.map((s) => s.const)).size === variants.length;
      const discriminator = Object.keys(variants[0]?.properties || {}).find(
        (key) =>
          variants.every(
            (s) =>
              s.type === "object" &&
              s.required?.includes(key) &&
              typeof s.properties?.[key]?.const === "string",
          ) &&
          new Set(variants.map((s) => s.properties[key].const)).size ===
            variants.length,
      );
      if (!literals && !discriminator)
        throw new Error(
          "Exclusive unions require disjoint required string tags",
        );
    }
    return `z.union([${variants.map((s) => validator(s, root)).join(",")}])`;
  }
  if (Array.isArray(schema.type))
    return `z.union([${schema.type.map((type) => validator({ ...schema, type }, root)).join(",")}])`;
  let code;
  switch (schema.type) {
    case "string":
      code = "z.string()";
      break;
    case "integer":
      code = "z.number().int()";
      break;
    case "number":
      code = "z.number()";
      break;
    case "boolean":
      code = "z.boolean()";
      break;
    case "null":
      code = "z.null()";
      break;
    case "array":
      code = `z.array(${validator(schema.items, root)})`;
      break;
    case "object": {
      if (schema.additionalProperties && !schema.properties) {
        only(schema, ["type", "additionalProperties"]);
        return `z.record(z.string(),${validator(schema.additionalProperties, root)})`;
      }
      if (schema.additionalProperties !== false)
        throw new Error("DTO object must reject unknown fields");
      code = `z.object({${Object.entries(schema.properties || {})
        .map(
          ([name, value]) =>
            `${JSON.stringify(name)}:${validator(value, root)}${schema.required?.includes(name) ? "" : ".optional()"}`,
        )
        .join(",")}}).strict()`;
      break;
    }
    default:
      throw new Error(
        "Unsupported source DTO schema: " + JSON.stringify(schema),
      );
  }
  for (const [key, method] of [
    ["minimum", "min"],
    ["maximum", "max"],
    ["minLength", "min"],
    ["maxLength", "max"],
    ["minItems", "min"],
    ["maxItems", "max"],
  ])
    if (
      schema[key] !== undefined &&
      ((["minimum", "maximum"].includes(key) &&
        ["integer", "number"].includes(schema.type)) ||
        (["minLength", "maxLength"].includes(key) &&
          schema.type === "string") ||
        (["minItems", "maxItems"].includes(key) && schema.type === "array"))
    )
      code += `.${method}(${schema[key]})`;
  if (schema.pattern && schema.type === "string")
    code += `.regex(new RegExp(${JSON.stringify(schema.pattern)}))`;
  if (schema.format !== undefined) {
    if (schema.type === "string" && schema.format === "uri") code += ".url()";
    else if (!(
      (schema.type === "integer" &&
        [
          "uint",
          "uint8",
          "uint16",
          "uint32",
          "uint64",
          "int8",
          "int16",
          "int32",
          "int64",
        ].includes(schema.format)) ||
      (schema.type === "number" &&
        ["float", "double"].includes(schema.format)) ||
      (schema.type === "null" &&
        [
          "uri",
          "uint",
          "uint8",
          "uint16",
          "uint32",
          "uint64",
          "int8",
          "int16",
          "int32",
          "int64",
          "float",
          "double",
        ].includes(schema.format))
    ))
      throw new Error("Unsupported source DTO format: " + schema.format);
  }
  for (const key of Object.keys(schema))
    if (
      ![
        "$schema",
        "$defs",
        "title",
        "description",
        "default",
        "format",
        "type",
        "properties",
        "required",
        "additionalProperties",
        "items",
        "minimum",
        "maximum",
        "minLength",
        "maxLength",
        "minItems",
        "maxItems",
        "pattern",
      ].includes(key)
    )
      throw new Error("Unsupported source DTO constraint: " + key);
  return code;
}
