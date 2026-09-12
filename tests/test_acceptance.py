import subprocess
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
BINARY = ROOT / "target" / "debug" / "banco-memoria"


def run_session(commands: list[str], working_directory: Path = ROOT) -> list[str]:
    process = subprocess.run(
        [str(BINARY)],
        cwd=working_directory,
        input="\n".join(commands) + "\n",
        text=True,
        capture_output=True,
        check=False,
    )
    if process.returncode != 0:
        raise AssertionError(
            f"process exited with {process.returncode}\n"
            f"stdout:\n{process.stdout}\nstderr:\n{process.stderr}"
        )

    normalized = process.stdout.replace("> ", "")
    return [line for line in normalized.splitlines() if line]


class BancoAcceptanceTests(unittest.TestCase):
    def test_full_official_script(self) -> None:
        commands = []
        expectations = []

        for raw_line in (ROOT / "casos_teste.txt").read_text(encoding="utf-8").splitlines():
            if raw_line.lstrip().startswith("#") or "->" not in raw_line:
                continue

            command, expected = raw_line.split("->", maxsplit=1)
            command = command.strip()
            expected = expected.strip()
            commands.append(command)

            if command and command != "EXIT":
                expectations.append(expected)

        output = run_session(commands)
        self.assertEqual(len(output), len(expectations))

        for actual, expected in zip(output, expectations, strict=True):
            if expected.startswith("ERRO"):
                self.assertTrue(actual.startswith("ERRO:"), (actual, expected))
            else:
                self.assertEqual(actual, expected)

    def test_cpf_validation_uniqueness_and_transactional_overwrite(self) -> None:
        output = run_session(
            [
                "ADD cpf_a 52998224725",
                "GET cpf_a",
                "ADD cpf_b 12345678900",
                "ADD cpf_b 11111111111",
                "ADD cpf_b 28746351973",
                "ADD cpf_c 28746351973",
                "ADD cpf_b 28746351973",
                "ADD cpf_b 00000000000",
                "GET cpf_b",
                "EXIT",
            ]
        )

        self.assertEqual(output[0:2], ["OK", "529.982.247-25"])
        self.assertTrue(output[2].startswith("ERRO:"))
        self.assertTrue(output[3].startswith("ERRO:"))
        self.assertEqual(output[4], "OK")
        self.assertIn("cpf_b", output[5])
        self.assertEqual(output[6], "OK")
        self.assertTrue(output[7].startswith("ERRO:"))
        self.assertEqual(output[8], "287.463.519-73")

    def test_calendar_validation_and_formatting(self) -> None:
        output = run_session(
            [
                "ADD data_a 2024-02-29",
                "GET data_a",
                "ADD data_b 2000-02-29",
                "GET data_b",
                "ADD data_c 1900-02-29",
                "ADD data_d 2100-02-29",
                "ADD data_e 2023-04-31",
                "ADD data_f 2023-1-5",
                "EXIT",
            ]
        )

        self.assertEqual(output[0:4], ["OK", "29/02/2024", "OK", "29/02/2000"])
        self.assertTrue(all(line.startswith("ERRO:") for line in output[4:]))

    def test_plain_values_commands_and_missing_keys(self) -> None:
        output = run_session(
            [
                "ADD nome_usuario Maria da Silva",
                "GET nome_usuario",
                "ADD cpfx 123",
                "GET cpfx",
                "GET missing",
                "GET",
                "ADD",
                "DELETE key",
                "add key value",
                "EXIT",
            ]
        )

        self.assertEqual(output[0:4], ["OK", "Maria da Silva", "OK", "123"])
        self.assertTrue(all(line.startswith("ERRO:") for line in output[4:]))

    def test_temperature_extension(self) -> None:
        output = run_session(
            [
                "ADD temperatura_sala 25.5",
                "GET temperatura_sala",
                "ADD temperatura_limite -273.15",
                "GET temperatura_limite",
                "ADD temperatura_sala -300",
                "GET temperatura_sala",
                "ADD temperatura_precisao 1.234",
                "EXIT",
            ]
        )

        self.assertEqual(output[0:4], [
            "OK",
            "25.50 °C = 77.90 °F",
            "OK",
            "-273.15 °C = -459.67 °F",
        ])
        self.assertTrue(output[4].startswith("ERRO:"))
        self.assertEqual(output[5], "25.50 °C = 77.90 °F")
        self.assertTrue(output[6].startswith("ERRO:"))

    def test_unknown_extension_works_without_recompiling(self) -> None:
        extension = """
local function success(value)
    return { ok = true, value = value }
end

local function failure(reason)
    return { ok = false, error = reason }
end

register_extension({
    prefix = "secret_",
    on_add = function(key, value)
        if #value < 3 then
            return failure("valor muito curto")
        end

        local normalized = string.upper(value)
        local existing_key = db_find_key_by_value(normalized, key)
        if existing_key ~= nil then
            return failure("valor já usado em " .. existing_key)
        end

        return success(normalized)
    end,
    on_get = function(_, value)
        return success("[" .. value .. "]")
    end,
})
"""

        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            extensions = directory / "extensions"
            extensions.mkdir()
            (extensions / "unknown.lua").write_text(extension, encoding="utf-8")

            output = run_session(
                [
                    "ADD secret_first abc",
                    "GET secret_first",
                    "ADD secret_second abc",
                    "ADD secret_short x",
                    "EXIT",
                ],
                directory,
            )

        self.assertEqual(output[0:2], ["OK", "[ABC]"])
        self.assertIn("secret_first", output[2])
        self.assertTrue(output[3].startswith("ERRO:"))


if __name__ == "__main__":
    unittest.main()
