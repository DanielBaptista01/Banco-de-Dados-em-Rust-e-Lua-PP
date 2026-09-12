local function success(value)
    return { ok = true, value = value }
end

local function failure(reason)
    return { ok = false, error = reason }
end

local function has_valid_decimal_format(value)
    return value:match("^%-?%d+$") ~= nil
        or value:match("^%-?%d+%.%d%d?$") ~= nil
end

local function validate_on_add(_, value)
    if not has_valid_decimal_format(value) then
        return failure("temperatura deve ser um número em Celsius com até duas casas decimais")
    end

    local celsius = tonumber(value)
    if celsius < -273.15 then
        return failure("temperatura não pode ser inferior ao zero absoluto (-273,15 °C)")
    end

    return success(string.format("%.2f", celsius))
end

local function convert_on_get(_, value)
    local celsius = tonumber(value)
    local fahrenheit = celsius * 9 / 5 + 32
    return success(string.format("%.2f °C = %.2f °F", celsius, fahrenheit))
end

register_extension({
    prefix = "temperatura_",
    on_add = validate_on_add,
    on_get = convert_on_get,
})
