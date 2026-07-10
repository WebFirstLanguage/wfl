# Media Module

The Media module lets WFL programs inspect and resize images natively — decoding
and re-encoding happen inside the language, so you never have to shell out to an
external tool like ImageMagick just to make a thumbnail.

Images move through WFL as **binary data** (the same `Binary` values you get from
reading a file as binary or from a binary HTTP body). You read the raw bytes,
hand them to a media function, and get bytes back that you can write to a file or
send as a web response.

Supported formats: **PNG, JPEG, GIF, BMP, and WebP**. The format is detected
automatically from the image's contents — you never pass it in.

## Functions

### image_dimensions

**Purpose:** Read an image's pixel width and height without fully decoding it.

**Signature:**
```wfl
image_dimensions of image_data
```

**Parameters:**
- `image_data` (Binary) - The raw bytes of an image.

**Returns:** A map with two number fields, `width` and `height`.

**Example:**
```wfl
open file at "photo.jpg" for reading binary as source
store data as read binary from source
close file source

store size as image_dimensions of data
display "This image is " with size["width"] with " by " with size["height"]
```

**Notes:**
- Only the image header is read, so this is cheap even for large images.
- Errors clearly if the bytes are not a recognized image.

---

### resize_image

**Purpose:** Resize an image to fit within a box, preserving its aspect ratio.

**Signature:**
```wfl
resize_image of image_data and width and height
```

**Parameters:**
- `image_data` (Binary) - The raw bytes of the source image.
- `width` (Number) - Maximum width of the result, in pixels (a whole number ≥ 1).
- `height` (Number) - Maximum height of the result, in pixels (a whole number ≥ 1).

**Returns:** Binary - The resized image, re-encoded in the **same format** as the
source.

**Behavior:**
- The image is scaled to fit **inside** the `width` × `height` box while keeping
  its original proportions, so it is never stretched or distorted. The result is
  never larger than the box on either axis.
- A square source fitted into a square box comes out at exactly that size; a wide
  source is limited by width, a tall source by height.
- High-quality (Lanczos3) resampling is used, which is well suited to producing
  thumbnails.

**Example — make a thumbnail and save it:**
```wfl
open file at "photo.jpg" for reading binary as source
store original as read binary from source
close file source

store thumbnail as resize_image of original and 200 and 200

open file at "photo_thumb.jpg" for writing binary as out_file
write binary thumbnail into out_file
close file out_file
```

**Example — resize an upload before serving it (web server):**
```wfl
// `request_body` is the binary body of an image upload.
store avatar as resize_image of request_body and 128 and 128
respond to request with avatar and content_type mime_type of "avatar.png"
```

**Notes:**
- To force exact dimensions regardless of aspect ratio, first read the source
  size with `image_dimensions` and compute the box you want.
- Very large targets are rejected (each axis is capped, and the total pixel count
  is limited) to keep memory use bounded.
- Because JPEG has no transparency, resizing a JPEG that carries alpha flattens it
  to opaque RGB.

## See Also

- [Filesystem Module](filesystem-module.md) — reading and writing binary files.
- [Web Servers](../04-advanced-features/web-servers.md) — serving binary responses
  and using `mime_type`.
