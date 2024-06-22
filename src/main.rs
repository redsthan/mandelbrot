extern crate num;
extern crate image;

use num::Complex;
use std::str::FromStr;
use image::ColorType;
use image::png::PNGEncoder;
use std::fs::File;
use std::io::Write;

/// Cherche à savoir si `c` est dans l'ensemble de Mandelbrot, en vérifiant
/// si la suite de complexe `z_{n+1} = z_n^2 + c` ne diverge pas.
///
/// Si la suite diverge, renvoie `Some(i)`, où `i` est le nombre d'itérations
/// avant que la suite ne diverge. Si la suite ne diverge pas après `MAX_ITER`,
/// renvoie `None`.
fn escape_time(c: Complex<f64>, limit: u32) -> Option<u32> {
    let mut z = Complex { re: 0.0, im: 0.0 };
    for i in 0..limit {
        z = z * z + c;
        if z.norm_sqr() > 4.0 {
            return Some(i);
        }
    }
    None
}

/// Analyse de la chaîne s en tant que paire de coordonnées
/// du type "400x600" ou "1.0,0.5".
/// Le format doit être <gauche><sep><droite>, avec <sep> le
/// caractère fourni par le paramètre 'separator', et <gauche>
/// et <droite> des chaînes qui peuvent être analysées par T::from_str.
///
/// Si s est bien formé, renvoyer Some<(x, y)>. Si l'analyse
/// échoue, renvoyer None.
fn analy_paire<T: FromStr>(s: &str, separator: char) -> Option<(T, T)> {
    match s.find(separator) {
        None => None,
        Some(index) => {
            match (T::from_str(&s[..index]), T::from_str(&s[index + 1..])) {
                (Ok(l), Ok(r)) => Some((l, r)),
                _ => None
            }
        }
    }
}

#[test]
fn test_analy_paire() {
    assert_eq!(analy_paire::<i32>("", ','), None);
    assert_eq!(analy_paire::<i32>("10,", ','), None);
    assert_eq!(analy_paire::<i32>(",10", ','), None);
    assert_eq!(analy_paire::<i32>("10,20", ','), Some((10, 20)));
    assert_eq!(analy_paire::<i32>("10,20xy", ','), None);
    assert_eq!(analy_paire::<f64>("0.5x", 'x'), None);
    assert_eq!(analy_paire::<f64>("0.5x1.5", 'x'), Some((0.5, 1.5)));
}

/// Analyse une paire de nombres flottants séparés par une virgule
/// considérés comme un nombre complexe.
fn analy_complex(s: &str) -> Option<Complex<f64>> {
    match analy_paire(s, ',') {
        Some((re, im)) => Some(Complex { re, im }),
        None => None
    }
}

#[test]
fn test_analy_complex() {
    assert_eq!(analy_complex("1.25,-0.0625"), Some(Complex { re: 1.25, im: -0.0625 }));
    assert_eq!(analy_complex(",-0.0625"), None);
}

/// En partant de la ligne et de la colonne d'un pixel de l'image
/// de sortie, renvoyer le le point du plan complexe correspondant.
///
/// 'bords' est la paire pour la largeur et la hauteur de l'image
/// en pixels.
/// 'super_ga' et 'infer_dr' sont des points du plan complexe
/// délimitant la zone d'image.
fn pixel_en_point(bords: (usize, usize),
                  pixel: (usize, usize),
                  super_ga: Complex<f64>,
                  infer_dr: Complex<f64>)
    -> Complex<f64> 
{
    let (large, haute) = (infer_dr.re - super_ga.re, 
                          super_ga.im - infer_dr.im);
    Complex {
        re: super_ga.re + pixel.0 as f64 * large / bords.0 as f64,
        im: super_ga.im - pixel.1 as f64 * haute / bords.1 as f64
    }
}

#[test]
fn test_pixel_en_point() {
    assert_eq!(pixel_en_point((100, 100), (25, 75),
                              Complex { re: -1.0, im: 1.0 },
                              Complex { re: 1.0, im: -1.0 }),
               Complex { re: -0.5, im: -0.5 });
}

/// Production dans un tampon de pixels d'un rectangle Mandelbrot.

/// Bords indique la hauteur et la largeur du tampon pixels
/// qui contient un pixel en nuance de gris par octet.
/// Les variables super_ga et infer_dr correspondent aux angles
/// supérieur gauche et inférieur droit du rectangle du tampon.
fn render(pixels: &mut [u8],
          bords: (usize, usize),
          super_ga: Complex<f64>,
          infer_dr: Complex<f64>)
{
    assert!(pixels.len() == bords.0 * bords.1);
    
    for ligne in 0..bords.1 {
        for colonne in 0..bords.0 {
            let point = pixel_en_point(bords, (colonne, ligne),
                                       super_ga, infer_dr);
            pixels[ligne * bords.0 + colonne] =
                match escape_time(point, 255) {
                    None => 0,
                    Some(i) => 255 - i as u8
                };
        }
    }
}

/// Ecrit le tampon 'pixels', de dimensions 'bords', dans le
/// fichier 'nomfic'.
fn ecrire_image(nomfic: &str, pixels: &[u8], bords: (usize, usize))
    -> Result<(), std::io::Error>
{
    let sortie = File::create(nomfic)?;

    let encodeur = PNGEncoder::new(sortie);
    encodeur.encode(&pixels,
                    bords.0 as u32, bords.1 as u32,
                    ColorType::Gray(8))?;
    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() != 5 {
        writeln!(std::io::stderr(),
                 "Usage: mandelbrot NOMFIC PIXELS SUPGA INFDR")
            .unwrap();
        writeln!(std::io::stderr(),
                 "Exemple: {} mandel.png 1000x750 -1.20,0.35 -1,0.20",
                 args[0])
            .unwrap();
        std::process::exit(1);
    }

    let bords = analy_paire(&args[2], 'x')
        .expect("Impossible d'analyser les dimensions de l'image");
    let super_ga = analy_complex(&args[3])
        .expect("Impossible d'analyser le coin supérieur gauche");
    let infer_dr = analy_complex(&args[4])
        .expect("Impossible d'analyser le coin inférieur droit");

    let mut pixels = vec![0; bords.0 * bords.1];

     
    let exetrons = 32;
    let lig_par_bande = bords.1 / exetrons+1;

    {
        let bandes: Vec<&mut [u8]> = 
            pixels.chunks_mut(lig_par_bande * bords.0).collect();
        crossbeam::scope(|spawner| {
            for (i, bande) in bandes.into_iter().enumerate() {
                let top = lig_par_bande * i;
                let haute = bande.len() / bords.0;
                let bande_bords = (bords.0, haute);
                let bande_supg = pixel_en_point(bords, (0, top), super_ga, infer_dr);
                let bande_infd = pixel_en_point(bords, (bords.0, top + haute), super_ga, infer_dr);

                spawner.spawn(move || {
                    render(bande, bande_bords, bande_supg, bande_infd)
                });
            }
        });
    }

    ecrire_image(&args[1], &pixels, bords)
        .expect("Impossible d'écrire l'image");
}