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

- [ ] **make_kernel** - Создание filter kernel по имени
- [ ] **ocionamedtransform** - Named OCIO transform
- [ ] **color_count** - Подсчёт уникальных цветов

## Требуют внешних зависимостей

- [ ] **render_text** - Рендеринг текста (требует FreeType/fontconfig)
- [ ] **demosaic** - Bayer demosaicing (специализированный алгоритм)
- [ ] **make_texture** - Создание mipmapped текстур (специфично для рендеринга)

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
| fillholes_pushpull | ✅ | ✅ | ⬜ |
| make_kernel | ⬜ | ⬜ | ⬜ |
