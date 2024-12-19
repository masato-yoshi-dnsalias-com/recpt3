# recpt3
## Rust で記述された recpt1 互換のテレビチューナーリーダー/ARIB STD-B25 デコーダーです。
[recpt1](https://github.com/stz2012/recpt1) を Rust で書き直しています。（一部機能の削除、変更はしています）  
　削除機能  
　　recpt3　　 ：UDPストリーム配信機能  
　　checksignal：bell 鳴動機能  
　　recpt1ctl　 ：機能削除  
　変更機能  
 　　recpt3　：httpストリーム配信をマルチスレッドで複数配信が可能  
   　　ts_splliter：tsストリームファイルのSIDで分離してファイル出力機能を追加  

※ドライバー、本家やその他分家のものをお使い下さい。  
 PT1/PT2：http://sourceforge.jp/projects/pt1dvr/  
 PT3：https://github.com/m-tsudo/pt3  
 px4_drv（本家が更新停止状態なの更新を引き継いだフォーク版）：https://github.com/tsukumijima/px4_drv
 
libarib25は以下のものをお使い下さい。  
 libarib25：https://github.com/stz2012/libarib25  
※機能統合版(https://github.com/tsukumijima/libaribb25) を使う場合は関数が増えいるため、arib25.rsのARIB_STD_B25構造体のコメントを外してください。（テストはしてません）  

アースソフトPT3 と Plex PX-Q3PE5で動作確認しています。

## recpt3：録画コマンド
    recpt3 [--b25 [--round N] [--strip] [--EMM]] [--http portnumber] [--device devicefile] [--lnb voltage] [--sid SID1,SID2,...] channel rectime outfile
詳しいオプションは「recpt3 --help」を参照してください。  
recpt1と同様に、デバイス指定なしの場合は利用可能なデバイスを自動で割り当てます。  

## checksignal：チェックシグナルコマンド
    checksignal [--device devicefile] [--lnb voltage] channel  
詳しいオプションは「checksignal --help」を参照してください。  

## ts_splitter：MPEG2-TS SID分離コマンド
    ts_splitter --sid SID1,SID2,... infile outfile
詳しいオプションは「ts_splitter --help」を参照してください。  

# ビルド
ビルドするには Rust が必要です。  
Rust がインストールされていない場合は、Rustup をインストールしてください。  
## Ubuntu / Debian
	curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
上記のコマンドでRustupをインストールできます。  

## コンパイルとインストール
    bash install.sh

## 手動コンパイル

    cargo build --release --bin recpt3
    cargo build --release --bin checksignal
    cargo build --release --bin ts_splitter

## 手動インストール
    install target/release/recpt3 /usr/local/bin
    install target/release/checksignal /usr/local/bin
    install target/release/ts_splitter /usr/local/bin
