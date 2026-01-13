# VFX-RS Testing System

Система тестирования для верификации математической точности vfx-rs относительно PyOpenColorIO.

## Структура

```
tests/
├── README.md           # Этот файл
├── parity/             # Python тесты против OCIO
│   ├── conftest.py     # Pytest фикстуры и хелперы
│   ├── test_transfer_parity.py   # Transfer functions
│   ├── test_matrix_parity.py     # Матричные преобразования
│   ├── test_lut_parity.py        # LUT операции
│   ├── test_ops_parity.py        # Grading операции (CDL, etc.)
│   └── generate_golden.py        # Генератор golden hashes
└── golden/
    ├── hashes.json     # Golden reference hashes от OCIO
    ├── reference_images/
    └── expected_outputs/
```

## Быстрый старт

### 1. Установка зависимостей

```bash
pip install pytest numpy opencolorio
```

### 2. Генерация golden data

```bash
python tests/parity/generate_golden.py
```

Создаёт `tests/golden/hashes.json` с референсными хешами от OCIO.

### 3. Запуск Rust golden тестов

```bash
cargo test --package vfx-tests golden -- --nocapture
```

### 4. Запуск Python parity тестов

```bash
pytest tests/parity/ -v
```

## Типы тестов

### Golden Hash Tests (Rust)

Быстрые тесты без Python зависимостей. Сравнивают SHA256 хеши выходных данных.

```bash
# Все golden тесты
cargo test --package vfx-tests golden

# Только transfer functions
cargo test --package vfx-tests golden::transfer

# Только матрицы
cargo test --package vfx-tests golden::matrix

# Только CDL
cargo test --package vfx-tests golden::cdl
```

### Parity Tests (Python)

Полные тесты с живым сравнением против PyOpenColorIO.

```bash
# Все parity тесты
pytest tests/parity/ -v

# По категориям
pytest tests/parity/ -v -m transfer
pytest tests/parity/ -v -m matrix
pytest tests/parity/ -v -m lut
pytest tests/parity/ -v -m ops
```

## Tolerance

- **RTOL** (relative): `1e-4` — относительная погрешность
- **ATOL** (absolute): `1e-6` — абсолютная погрешность для значений около нуля

## Интерпретация результатов

### MATCH
Хеш vfx-rs совпадает с OCIO — реализации идентичны.

### MISMATCH
Хеши различаются. Возможные причины:
- Разные коэффициенты в формулах
- Разная точность констант
- Разный порядок операций
- Разное clamping/saturation

### Отладка расхождений

1. Сравнить статистику (min/max/mean) в `hashes.json`
2. Проверить формулы в OCIO source code
3. Построить график разницы для визуального анализа

## Добавление новых тестов

### Python parity test

```python
# tests/parity/test_new_feature.py
import pytest
from conftest import apply_ocio_cpu, assert_close, RTOL

@pytest.mark.new_feature
class TestNewFeature:
    def test_something(self, ocio, ocio_raw_config, test_values_standard):
        # OCIO reference
        transform = ocio.SomeTransform()
        processor = ocio_raw_config.getProcessor(transform)
        ocio_result = apply_ocio_cpu(processor, test_values_standard)
        
        # vfx-rs implementation
        vfx_result = ...  # your implementation
        
        assert_close(vfx_result, ocio_result, rtol=RTOL)
```

### Rust golden test

```rust
// В crates/vfx-tests/src/golden.rs
#[test]
fn test_new_feature() {
    let input = gray_ramp_256();  // или rgb_cube_8()
    let result = your_function(&input);
    let hash = compute_hash_f32(&result);
    
    println!("new_feature hash: {}", hash);
    
    // После генерации golden data:
    // if let Some(entry) = golden.tests.new_feature.get("name") {
    //     assert_eq!(hash, entry.hash);
    // }
}
```

### Добавление в generate_golden.py

```python
def generate_new_feature_hashes() -> dict:
    hashes = {}
    # ... generate reference data from OCIO
    return hashes

# В generate_all_hashes():
golden["tests"]["new_feature"] = generate_new_feature_hashes()
```

## CI/CD

Тесты предназначены для локального запуска. Для CI:

```yaml
# .github/workflows/test.yml
- name: Run golden tests
  run: cargo test --package vfx-tests golden
```

Python parity тесты требуют OpenColorIO и запускаются опционально.

## Troubleshooting

### "OpenColorIO not installed"
```bash
pip install opencolorio
```

### "No golden data found"
```bash
python tests/parity/generate_golden.py
```

### Все тесты MISMATCH
Проверьте версию OCIO:
```bash
python -c "import PyOpenColorIO; print(PyOpenColorIO.__version__)"
```
Golden data сгенерированы для OCIO 2.x.

## Ссылки

- [OpenColorIO Documentation](https://opencolorio.readthedocs.io/)
- [ASC CDL Specification](https://theasc.com/asc/asc-cdl)
- [ACES Documentation](https://docs.acescentral.com/)
