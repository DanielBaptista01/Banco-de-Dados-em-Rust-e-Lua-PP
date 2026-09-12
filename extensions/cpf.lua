local function success(value)
    return { ok = true, value = value }
end

local function failure(reason)
    return { ok = false, error = reason }
end

local function calculate_digit(value, length, initial_weight)
    local sum = 0
    for index = 1, length do
        local digit = tonumber(value:sub(index, index))
        sum = sum + digit * (initial_weight - index + 1)
    end

    local remainder = sum % 11
    if remainder < 2 then
        return 0
    end
    return 11 - remainder
end

local function validate_on_add(key, value)
    if #value ~= 11 then
        return failure("CPF deve conter exatamente 11 dígitos")
    end

    if not value:match("^%d+$") then
        return failure("CPF deve conter apenas números, sem formatação")
    end

    if value == string.rep(value:sub(1, 1), 11) then
        return failure("CPF não pode ser uma sequência de dígitos repetidos")
    end

    local first_digit = calculate_digit(value, 9, 10)
    local second_digit = calculate_digit(value, 10, 11)

    if first_digit ~= tonumber(value:sub(10, 10))
        or second_digit ~= tonumber(value:sub(11, 11)) then
        return failure("CPF com dígito verificador inválido")
    end

    local existing_key = db_find_key_by_value(value, key)
    if existing_key ~= nil then
        return failure("CPF já cadastrado na chave " .. existing_key)
    end

    return success(value)
end

local function format_on_get(_, value)
    local formatted = value:sub(1, 3)
        .. "." .. value:sub(4, 6)
        .. "." .. value:sub(7, 9)
        .. "-" .. value:sub(10, 11)
    return success(formatted)
end

register_extension({
    prefix = "cpf_",
    on_add = validate_on_add,
    on_get = format_on_get,
})

