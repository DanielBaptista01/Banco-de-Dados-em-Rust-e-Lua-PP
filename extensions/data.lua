local function success(value)
    return { ok = true, value = value }
end

local function failure(reason)
    return { ok = false, error = reason }
end

local function is_leap_year(year)
    return year % 400 == 0 or (year % 4 == 0 and year % 100 ~= 0)
end

local function validate_on_add(_, value)
    if #value ~= 10
        or value:sub(5, 5) ~= "-"
        or value:sub(8, 8) ~= "-"
        or not value:sub(1, 4):match("^%d%d%d%d$")
        or not value:sub(6, 7):match("^%d%d$")
        or not value:sub(9, 10):match("^%d%d$") then
        return failure("data inválida: use exatamente o formato aaaa-mm-dd")
    end

    local year = tonumber(value:sub(1, 4))
    local month = tonumber(value:sub(6, 7))
    local day = tonumber(value:sub(9, 10))

    if month < 1 or month > 12 then
        return failure("data inválida: mês fora da faixa de 01 a 12")
    end

    local days_in_month = { 31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31 }
    if month == 2 and is_leap_year(year) then
        days_in_month[2] = 29
    end

    if day < 1 or day > days_in_month[month] then
        return failure("data inválida: dia fora da faixa para o mês informado")
    end

    return success(value)
end

local function format_on_get(_, value)
    local formatted = value:sub(9, 10)
        .. "/" .. value:sub(6, 7)
        .. "/" .. value:sub(1, 4)
    return success(formatted)
end

register_extension({
    prefix = "data_",
    on_add = validate_on_add,
    on_get = format_on_get,
})

