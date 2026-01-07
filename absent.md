# Отсутствующие функции OIIO

## Приоритет: Высокий

- [x] **rotate** - Поворот на произвольный угол с интерполяцией (уже реализован)
- [x] **fit** - Resize с сохранением пропорций (уже реализован)
- [x] **normalize** - Нормализация RGB векторов

## Приоритет: Средний

- [x] **circular_shift** - Циклический сдвиг пикселей (уже реализован)
- [x] **reorient** - Автоориентация по EXIF метаданным
- [x] **resample** - Быстрый nearest-neighbor resize (уже реализован)
- [x] **zover** - Z-depth compositing
- [x] **fillholes_pushpull** - Заполнение дыр в альфа-канале

## Приоритет: Низкий

- [x] **make_kernel** - Создание filter kernel по имени
- [x] **ocionamedtransform** - Named OCIO transform
- [x] **color_count** - Подсчёт уникальных цветов

## Требуют внешних зависимостей

- [x] **render_text** - Рендеринг текста (cosmic-text, feature "text")
- [x] **demosaic** - Bayer demosaicing (RGGB/BGGR/GRBG/GBRG, bilinear/VNG)
- [x] **make_texture** - Создание mipmapped текстур (Box/Bilinear/Lanczos/Kaiser)

---

## Прогресс

| Функция | Rust | Python | Тесты |
|---------|------|--------|-------|
| rotate | ✅ | ✅ | ⬜ |
| fit | ✅ | ✅ | ⬜ |
| normalize | ✅ | ✅ | ⬜ |
| circular_shift | ✅ | ✅ | ⬜ |
| reorient | ✅ | ✅ | ⬜ |
| resample | ✅ | ✅ | ⬜ |
| zover | ✅ | ✅ | ⬜ |
| fillholes_pushpull | ✅ | ✅ | ✅ |
| make_kernel | ✅ | ✅ | ⬜ |
| ocionamedtransform | ✅ | ✅ | ⬜ |
| color_count | ✅ | ✅ | ⬜ |
| render_text | ✅ | ✅ | ✅ |
| demosaic | ✅ | ✅ | ✅ |
| make_texture | ✅ | ✅ | ✅ |
