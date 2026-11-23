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
  if (schema.$ref) {
    only(schema, ["$ref"]);
    const key = schema.$ref.replace(/^#\/\$defs\//, "");
    if (!root.$defs?.[key]) throw new Error("Unknown DTO schema reference");
    return validator(root.$defs[key], root);
  }
  if (schema.const !== undefined) {
    only(schema, ["const", "type"]);
    return `z.literal(${JSON.stringify(schema.const)})`;
  }
  if (schema.enum) {
    only(schema, ["enum", "type"]);
    return `z.enum(${JSON.stringify(schema.enum)})`;
  }
  const variants = schema.anyOf || schema.oneOf;
  if (variants) {
    only(schema, [schema.anyOf ? "anyOf" : "oneOf"]);
    if (
      schema.oneOf &&
      (!variants.every((s) => typeof s.const === "string") ||
        new Set(variants.map((s) => s.const)).size !== variants.length)
    )
      throw new Error("Only disjoint string literals support exclusive unions");
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
